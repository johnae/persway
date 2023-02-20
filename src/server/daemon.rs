use super::controller::{self, Controller};
use crate::commands::PerswayCommand;
use crate::Args;
use crate::{commands::DaemonArgs, layout::WorkspaceLayout, utils};
use anyhow::{anyhow, Result};
use async_std::os::unix::net::{UnixListener, UnixStream};
use async_std::prelude::*;
use async_std::task;
use clap::Parser;
use futures::channel::mpsc;
use futures::SinkExt;
use futures::{select, stream::StreamExt};
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use std::cell::RefCell;
use std::process::exit;
use std::rc::Rc;
use std::sync::Arc;
use swayipc_async::{Connection, Event, EventType, WindowEvent};

pub type Sender<T> = mpsc::UnboundedSender<T>;
pub type Receiver<T> = mpsc::UnboundedReceiver<T>;

pub enum Message {
    WindowEvent(Box<WindowEvent>),
    CommandEvent(PerswayCommand),
}

enum ClientCommand {
    StackFocusNext,
    StackFocusPrev,
    StackSwapVisible,
    StackMainRotatePrev,
    StackMainRotateNext,
    ChangeLayout { layout: WorkspaceLayout },
}

pub struct Daemon {
    on_exit: Option<String>,
    socket_path: String,
    controller: Controller,
}

impl Daemon {
    pub fn new(args: DaemonArgs, socket_path: Option<String>) -> Daemon {
        let socket_path = utils::get_socket_path(socket_path);
        match args {
            DaemonArgs {
                default_layout,
                workspace_renaming,
                on_window_focus,
                on_window_focus_leave,
                on_exit,
                ..
            } => Daemon {
                socket_path,
                on_exit,
                controller: Controller::new(
                    default_layout,
                    workspace_renaming,
                    on_window_focus,
                    on_window_focus_leave,
                ),
            },
        }
    }

    async fn handle_signals(signals: Signals) {
        let mut signals = signals.fuse();
        //let args = Cli::from_args();
        //let on_exit = args.on_exit;
        let on_exit = Some("");
        while let Some(signal) = signals.next().await {
            match signal {
                SIGHUP | SIGINT | SIGQUIT | SIGTERM => {
                    let mut commands = Connection::new().await.unwrap();
                    if let Some(exit_cmd) = on_exit {
                        log::debug!("{}", exit_cmd);
                        commands.run_command(exit_cmd).await.unwrap();
                    }
                    exit(0)
                }
                _ => unreachable!(),
            }
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM])?;
        let _handle = signals.handle();
        let _signals_task = async_std::task::spawn(Self::handle_signals(signals));

        let subs = [EventType::Window];
        let mut sway_events = Connection::new().await?.subscribe(&subs).await?.fuse();

        match async_std::fs::remove_file(&self.socket_path).await {
            Ok(()) => log::debug!("Removed stale socket {}", &self.socket_path),
            Err(e) => log::error!(
                "Unable to remove stale socket: {}\n{:?}",
                &self.socket_path,
                e
            ),
        };

        let listener = UnixListener::bind(&self.socket_path).await?;
        let mut incoming = listener.incoming().fuse();

        let (mut sender, mut receiver) = mpsc::unbounded();
        let mut receiver = receiver.fuse();

        loop {
            select! {
                event = sway_events.next() => {
                    if let Some(event) = event {
                        match event? {
                            Event::Window(event) => {
                                sender.send(Message::WindowEvent(event)).await?;
                            },
                            _ => unreachable!(),
                        }
                    }
                },
                stream = incoming.next() => {
                    if let Some(stream) = stream {
                        let stream = stream?;
                        log::debug!("Accepting connection from: {:?}", stream.peer_addr()?);
                        let _handle = task::spawn(Self::connection_loop(stream, sender.clone()));
                    }
                },
                message = receiver.next() => {
                    if let Some(message) = message {
                        match message {
                            Message::WindowEvent(event) => self.controller.handle_event(event).await?,
                            Message::CommandEvent(command) => self.controller.handle_command(command).await?,
                        }
                    }
                }
                complete => panic!("Stream-processing stopped unexpectedly"),
            }
        }
    }

    async fn handle_message(msg: PerswayCommand) -> Result<()> {
        log::debug!("handle msg: {:?}", msg);
        Ok(())
    }

    async fn connection_loop(mut stream: UnixStream, mut sender: Sender<Message>) -> Result<()> {
        let mut message = String::new();
        log::debug!("reading incoming msg");
        match stream.read_to_string(&mut message).await {
            Ok(_) => {
                log::debug!("got message: {}", message);
                log::debug!("writing success message back to client");
                let args = match Args::try_parse_from(message.split_ascii_whitespace()) {
                    Ok(args) => args,
                    Err(e) => {
                        log::error!("unknown message: {}\n{}", message, e);
                        return Err(anyhow!("unknown message"));
                    }
                };
                sender.send(Message::CommandEvent(args.command)).await?;
                stream.write_all(b"success\n").await?;
            }
            Err(e) => {
                log::error!("Invalid UTF-8 sequence: {}", e);
                log::debug!("writing failure message back to client");
                stream.write_all(b"fail: invalid utf-8 sequence").await?;
                stream.write_all(b"\n").await?;
            }
        }
        Ok(())
    }
}
