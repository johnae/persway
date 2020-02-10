use signal_hook::{iterator::Signals, SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use std::{process::exit, thread};
use structopt::StructOpt;
use swayipc::async_std;
use swayipc::async_std::stream::StreamExt;
use swayipc::reply::Event;
use swayipc::reply::{WindowChange, Workspace};
use swayipc::{Connection, EventType, Fallible};

#[derive(StructOpt)]
struct Cli {
    #[structopt(short = "o", long = "opacity", default_value = "0.78")]
    opacity: f64,
}

fn handle_signals() {
    let signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM]).unwrap();
    signals.forever().next();
    async_std::task::block_on(async {
        let mut commands = Connection::new().await.unwrap();
        commands.run_command("[tiling] opacity 1").await.unwrap();
    });
    exit(0)
}

async fn get_focused_workspace(conn: &mut Connection) -> Fallible<Workspace> {
    let mut ws = conn.get_workspaces().await?.into_iter();
    Ok(ws
        .find(|w| w.focused)
        .expect("no focused workspace, shouldn't happen"))
}

#[async_std::main]
async fn main() -> Fallible<()> {
    let args = Cli::from_args();
    thread::spawn(handle_signals);

    let subscriptions = [EventType::Window];
    let mut events = Connection::new().await?.subscribe(&subscriptions).await?;
    let mut commands = Connection::new().await?;

    while let Some(event) = events.next().await {
        match event? {
            Event::Window(wevent) => match wevent.change {
                WindowChange::Focus => {
                    let cmd = format!("[tiling] opacity {}; opacity 1", args.opacity);
                    commands.run_command(&cmd).await?;
                    let app_id = wevent.container.app_id;
                    let window_properties = wevent.container.window_properties;
                    let app_name = app_id.unwrap_or_else(|| {
                        window_properties
                            .map(|props| props.class)
                            .or(Some("Unknown".to_string()))
                            .unwrap()
                    });
                    let current_ws = get_focused_workspace(&mut commands).await?;
                    let num = current_ws
                        .name
                        .split(": ")
                        .next()
                        .unwrap_or(&current_ws.name);
                    let newname = format!("{}: {}", num, app_name.to_lowercase());
                    let cmd = format!("rename workspace to {}", newname);
                    commands.run_command(&cmd).await?;
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }

    unreachable!();
}
