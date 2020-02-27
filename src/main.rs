use failure::err_msg;
use signal_hook::{iterator::Signals, SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use std::{process::exit, thread};
use structopt::StructOpt;
use swayipc::async_std;
use swayipc::async_std::stream::StreamExt;
use swayipc::reply::Event;
use swayipc::reply::{NodeLayout, NodeType, WindowChange, WindowEvent, Workspace};
use swayipc::{Connection, EventType, Fallible};

#[derive(StructOpt)]
/// I am Persway. A friendly daemon.
///
/// I talk to the Sway Compositor and persuade it to do little evil things.
/// Give me an option and see what it brings.
struct Cli {
    /// Set the level of opacity to give non-focused containers
    #[structopt(short = "o", long = "opacity", default_value = "0.78")]
    opacity: f64,
    /// Enable autolayout, alternating between horizontal and vertical
    /// somewhat reminiscent of the Awesome WM.
    #[structopt(short = "a", long = "autolayout")]
    autolayout: bool,
    /// Enable automatic workspace renaming based on what is running
    /// in the workspace (eg. application name).
    #[structopt(short = "w", long = "workspace-renaming")]
    workspace_renaming: bool,
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
    let focused = tree
        .find_focused_as_ref(|n| n.focused)
        .ok_or(err_msg("No focused node"))?;
    let parent = tree
        .find_focused_as_ref(|n| n.nodes.iter().any(|n| n.focused))
        .ok_or(err_msg("No parent"))?;
    let is_floating = focused.node_type == NodeType::FloatingCon;
    let is_full_screen = focused.percent.unwrap_or(1.0) > 1.0;
    let is_stacked = parent.layout == NodeLayout::Stacked;
    let is_tabbed = parent.layout == NodeLayout::Tabbed;
    if !is_floating && !is_full_screen && !is_stacked && !is_tabbed {
        let cmd = if focused.rect.height > focused.rect.width {
            "split v"
        } else {
            "split h"
        };
        conn.run_command(cmd).await?;
    };

    Ok(())
}

async fn get_focused_workspace(conn: &mut Connection) -> Fallible<Workspace> {
    let mut ws = conn.get_workspaces().await?.into_iter();
    Ok(ws
        .find(|w| w.focused)
        .expect("no focused workspace, shouldn't happen"))
}

async fn rename_workspace(event: &Box<WindowEvent>, conn: &mut Connection) -> Fallible<()> {
    let current_ws = get_focused_workspace(conn).await?;
    let ws_num = current_ws
        .name
        .split(": ")
        .next()
        .unwrap_or(&current_ws.name);

    let app_id = event.container.app_id.as_ref();
    let window_properties = event.container.window_properties.as_ref();
    let app_name = app_id.map_or_else(
        || window_properties.and_then(|p| Some(&p.class)),
        |name| Some(name),
    );

    if let Some(app_name) = app_name {
        let newname = format!("{}: {}", ws_num, app_name.to_lowercase());
        let cmd = format!("rename workspace to {}", newname);
        conn.run_command(&cmd).await?;
    }
    Ok(())
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

                    if args.workspace_renaming {
                        if let Err(e) = rename_workspace(&wevent, &mut commands).await {
                            println!("workspace rename err: {}", e);
                        }
                    }

                    if args.autolayout {
                        if let Err(e) = autolayout(&mut commands).await {
                            println!("autolayout err: {}", e);
                        };
                    }
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }

    unreachable!();
}
