use derive_more::{Display, From};
use i3ipc::{
    event::{inner::WindowChange, Event},
    reply::Command,
    I3Connection, I3EventListener, Subscription,
};
use signal_hook::{iterator::Signals, SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use std::{process::exit, thread};
use structopt::StructOpt;

#[derive(Debug, Display, From)]
enum PerswayError {
    I3Msg(i3ipc::MessageError),
    I3Establish(i3ipc::EstablishError),
    Io(std::io::Error),
}

impl std::error::Error for PerswayError {}

type Result<T> = std::result::Result<T, PerswayError>;

#[derive(StructOpt)]
struct Cli {
    #[structopt(short = "o", long = "opacity", default_value = "0.78")]
    opacity: f64,
}

fn update_opacity(ipc: &mut I3Connection, opacity: f64) -> Result<Command> {
    let cmd = format!("[tiling] opacity {}; opacity 1", opacity);
    Ok(ipc.run_command(&cmd)?)
}

fn reset_opacity(ipc: &mut I3Connection) -> Result<Command> {
    Ok(ipc.run_command("[tiling] opacity 1")?)
}

fn handle_signals() -> Result<()> {
    let mut conn = I3Connection::connect()?;
    let signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM])?;
    signals.forever().next();
    reset_opacity(&mut conn)?;
    exit(0)
}

fn main() -> Result<()> {
    let args = Cli::from_args();
    thread::spawn(handle_signals);
    let mut conn = I3Connection::connect()?;
    let mut listener = I3EventListener::connect()?;
    listener.subscribe(&[Subscription::Window])?;

    reset_opacity(&mut conn)?;
    for event in listener.listen() {
        match event? {
            Event::WindowEvent(info) => {
                if let WindowChange::Focus = info.change {
                    update_opacity(&mut conn, args.opacity)?;
                }
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}
