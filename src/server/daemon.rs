use super::message_handler::MessageHandler;
use crate::commands::PerswayCommand;
use crate::layout::WorkspaceLayout;
use crate::Args;
use crate::{commands::DaemonArgs, utils};
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
use std::process::exit;
use swayipc_async::{Connection, Event, EventType, WindowEvent};

pub type Sender<T> = mpsc::UnboundedSender<T>;

pub enum Message {
    WindowEvent(Box<WindowEvent>),
    CommandEvent(PerswayCommand),
}

pub struct Daemon {
    on_exit: Option<String>,
    socket_path: String,
    message_handler: MessageHandler,
}

impl Daemon {
    pub fn new(args: DaemonArgs, socket_path: Option<String>) -> Daemon {
        let socket_path = utils::get_socket_path(socket_path);
        match args {
            DaemonArgs {
                default_layout,
                stack_main_default_size,
                stack_main_default_stack_layout,
                workspace_renaming,
                on_window_focus,
                on_window_focus_leave,
                on_exit,
                ..
            } => {
                let default_layout = match default_layout {
                    WorkspaceLayout::StackMain { .. } => WorkspaceLayout::StackMain {
                        size: stack_main_default_size,
                        stack_layout: stack_main_default_stack_layout,
                    },
                    _ => default_layout,
                };
                Daemon {
                    socket_path,
                    on_exit,
                    message_handler: MessageHandler::new(
                        default_layout,
                        workspace_renaming,
                        on_window_focus,
                        on_window_focus_leave,
                    ),
                }
            }
        }
    }

    async fn handle_signals(signals: Signals, on_exit: Option<String>) {
        let mut signals = signals.fuse();
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
        let _signals_task =
            async_std::task::spawn(Self::handle_signals(signals, self.on_exit.clone()));

        let subs = [EventType::Window];
        let mut sway_events = Connection::new().await?.subscribe(&subs).await?.fuse();

        match async_std::fs::remove_file(&self.socket_path).await {
            Ok(()) => log::debug!("Removed stale socket {}", &self.socket_path),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => log::debug!(
                    "Couldn't remove stale socket {} as the file didn't exist",
                    &self.socket_path
                ),
                _ => {
                    log::error!(
                        "Unable to remove stale socket: {}\n{:?}",
                        &self.socket_path,
                        e
                    )
                }
            },
        };

        let listener = UnixListener::bind(&self.socket_path).await?;
        let mut incoming = listener.incoming().fuse();

        let (mut sender, receiver) = mpsc::unbounded();
        let mut receiver = receiver.fuse();

        loop {
            select! {
                event = sway_events.select_next_some() => {
                        match event? {
                            Event::Window(event) => {
                                log::debug!("select: sway event sending through channel");
                                sender.send(Message::WindowEvent(event)).await?;
                                log::debug!("select: sway event sent through channel");
                            },
                            _ => unreachable!(),
                        }
                },
                stream = incoming.select_next_some() => {
                        let stream = stream?;
                        log::debug!("select: accepting connection from: {:?}", stream.peer_addr()?);
                        let _handle = task::spawn(Self::connection_loop(stream, sender.clone()));
                        log::debug!("select: connection handled");
                },
                message = receiver.select_next_some() => {
                    log::debug!("select: received message");
                    match message {
                        Message::WindowEvent(event) => {
                          log::debug!("select: handling message window event");
                          self.message_handler.handle_event(event).await?;
                          log::debug!("select: handled message window event");
                        },
                        Message::CommandEvent(command) => {
                          log::debug!("select: handling message command event");
                          self.message_handler.handle_command(command).await?;
                          log::debug!("select: handled message command event");
                        }
                    };
                    log::debug!("select: handled message");
                }
                complete => panic!("Stream-processing stopped unexpectedly"),
            }
        }
    }

    async fn connection_loop(mut stream: UnixStream, mut sender: Sender<Message>) -> Result<()> {
        let mut message = String::new();
        log::debug!("reading incoming msg");
        match stream.read_to_string(&mut message).await {
            Ok(_) => {
                log::debug!("got message: {}", message);
                let args = match Args::try_parse_from(message.split_ascii_whitespace()) {
                    Ok(args) => args,
                    Err(e) => {
                        log::error!("unknown message: {}\n{}", message, e);
                        return Err(anyhow!("unknown message"));
                    }
                };
                log::debug!("sending command through channel");
                sender.send(Message::CommandEvent(args.command)).await?;
                log::debug!("writing success message back to client");
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
