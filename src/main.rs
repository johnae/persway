use signal_hook::{iterator::Signals, SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use std::{process::exit, thread};
use structopt::StructOpt;
use swayipc::async_std;
use swayipc::async_std::stream::StreamExt;
use swayipc::reply::Event;
use swayipc::reply::{Node, NodeLayout, NodeType, WindowChange, Workspace};
use swayipc::{Connection, EventType, Fallible};

#[derive(StructOpt)]
struct Cli {
    #[structopt(short = "o", long = "opacity", default_value = "0.78")]
    opacity: f64,
    #[structopt(short = "a", long = "autolayout")]
    autolayout: bool,
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

async fn autolayout(conn: &mut Connection) -> Fallible<()> {
    let tree = conn.get_tree().await?;
    let focused: &Node = tree
        .find_focused_as_ref(|n| n.focused)
        .expect("Expected a node to be focused");
    let parent: &Node = tree
        .find_focused_as_ref(|n| n.nodes.iter().any(|n| n.focused))
        .expect("Expected to find a parent");
    let is_floating = focused.node_type == NodeType::FloatingCon;
    let is_full_screen = focused.percent.unwrap_or(1.0) > 1.0;
    let is_stacked = parent.layout == NodeLayout::Stacked;
    let is_tabbed = parent.layout == NodeLayout::Tabbed;
    let change_split = !is_floating && !is_full_screen && !is_stacked && !is_tabbed;
    if change_split {
        if focused.rect.height > focused.rect.width {
            conn.run_command("split v").await?;
        } else {
            conn.run_command("split h").await?;
        }
    };

    Ok(())
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
                    if args.autolayout {
                        autolayout(&mut commands).await?;
                    };
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }

    unreachable!();
}
