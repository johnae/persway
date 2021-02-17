use anyhow::{anyhow, Result};
use async_std::prelude::*;
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use std::process::exit;
use structopt::StructOpt;
use swayipc_async::{
    Connection, Event, EventType, NodeLayout, NodeType, WindowChange, WindowEvent, Workspace,
};

#[derive(StructOpt)]
/// I am Persway. A friendly daemon.
///
/// I talk to the Sway Compositor and persuade it to do little evil things.
/// Give me an option and see what it brings.
struct Cli {
    /// Enable autolayout, alternating between horizontal and vertical
    /// somewhat reminiscent of the Awesome WM.
    #[structopt(short = "a", long = "autolayout")]
    autolayout: bool,
    /// Enable automatic workspace renaming based on what is running
    /// in the workspace (eg. application name).
    #[structopt(short = "w", long = "workspace-renaming")]
    workspace_renaming: bool,
    /// Called when window comes into focus. To automatically set the opacity of
    /// all other windows to 0.8 for example, you would set this to:
    ///
    /// [tiling] opacity 0.8; opacity 1
    ///
    /// Eg. set all tiling windows to opacity 0.8 but set the currently focused window to opacity 1.
    /// Or if you want to skip some applications - in this case firefox - you would do something like:
    ///
    /// [tiling] opacity 0.8; [app_id="firefox"] opacity 1; opacity 1
    #[structopt(short = "f", long = "on-window-focus")]
    on_window_focus: Option<String>,
    /// Called when persway exits. This can be used to reset any opacity changes
    /// or other settings when persway exits. For example, if changing the opacity
    /// on window focus, you would probably want to reset that on exit like this:
    ///
    /// [tiling] opacity 1
    ///
    /// Eg. set all tiling windows to opacity 1
    #[structopt(short = "e", long = "on-exit")]
    on_exit: Option<String>,
}

async fn handle_signals(signals: Signals) {
    let mut signals = signals.fuse();
    let args = Cli::from_args();
    let on_exit = args.on_exit.unwrap_or(String::from(""));
    while let Some(signal) = signals.next().await {
        match signal {
            SIGHUP | SIGINT | SIGQUIT | SIGTERM => {
                let mut commands = Connection::new().await.unwrap();
                commands.run_command(format!("{}", on_exit)).await.unwrap();
                exit(0)
            }
            _ => unreachable!(),
        }
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    let on_window_focus = args.on_window_focus.unwrap_or(String::from(""));

    let signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM])?;
    let handle = signals.handle();
    let signals_task = async_std::task::spawn(handle_signals(signals));

    let mut commands = Connection::new().await?;
    let subs = [EventType::Window];
    let mut events = Connection::new().await?.subscribe(&subs).await?;
    while let Some(event) = events.next().await {
        match event? {
            Event::Window(event) => match event.change {
                WindowChange::Focus => {
                    commands.run_command(format!("{}", on_window_focus)).await?;
                    if args.workspace_renaming {
                        if let Err(e) = rename_workspace(&event, &mut commands).await {
                            println!("workspace rename err: {}", e);
                        }
                    };

                    if args.autolayout {
                        if let Err(e) = autolayout(&mut commands).await {
                            println!("autolayout err: {}", e);
                        };
                    };
                }
                WindowChange::Close => {
                    if args.workspace_renaming {
                        if let Err(e) = rename_workspace(&event, &mut commands).await {
                            println!("workspace rename err: {}", e);
                        }
                    };
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }

    handle.close();
    signals_task.await;
    Ok(())
}

async fn autolayout(conn: &mut Connection) -> Result<()> {
    let tree = conn.get_tree().await?;
    let focused = tree
        .find_focused_as_ref(|n| n.focused)
        .ok_or(anyhow!("No focused node"))?;
    let parent = tree
        .find_focused_as_ref(|n| n.nodes.iter().any(|n| n.focused))
        .ok_or(anyhow!("No parent"))?;
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

async fn get_focused_workspace(conn: &mut Connection) -> Result<Workspace> {
    let mut ws = conn.get_workspaces().await?.into_iter();
    ws.find(|w| w.focused)
        .ok_or(anyhow!("No focused workspace"))
}

async fn rename_workspace(event: &Box<WindowEvent>, conn: &mut Connection) -> Result<()> {
    let current_ws = get_focused_workspace(conn).await?;
    let ws_num = current_ws
        .name
        .split(": ")
        .next()
        .unwrap_or(&current_ws.name);

    if current_ws.focus.len() == 0 {
        let cmd = format!("rename workspace to {}", ws_num);
        conn.run_command(&cmd).await?;
        return Ok(());
    }

    let app_id = event.container.app_id.as_ref();
    let window_properties = event.container.window_properties.as_ref();
    let app_name = app_id.map_or_else(
        || window_properties.and_then(|p| p.class.as_ref()),
        |name| Some(name),
    );

    if let Some(app_name) = app_name {
        let newname = format!(
            "{}: {}",
            ws_num,
            app_name
                .trim_start_matches('-')
                .trim_end_matches('-')
                .to_lowercase()
        );
        let cmd = format!("rename workspace to {}", newname);
        conn.run_command(&cmd).await?;
    };
    Ok(())
}
