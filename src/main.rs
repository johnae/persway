use anyhow::Result;
use env_logger::Env;
mod client;
mod commands;
mod layout;
mod node_ext;
mod server;
use clap::Parser;
mod utils;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[command(subcommand)]
    command: commands::PerswayCommand,
    /// Path to control socket.
    /// Defaults to <XDG_RUNTIME_DIR>/persway-<WAYLAND_DISPLAY>.sock
    #[arg(long)]
    socket_path: Option<String>,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    match args.command {
        commands::PerswayCommand::Daemon(daemon_args) => {
            server::daemon::Daemon::new(daemon_args, args.socket_path)
                .run()
                .await?
        }
        _ => {
            client::send(
                args.socket_path,
                &std::env::args().into_iter().collect::<Vec<_>>().join(" "),
            )
            .await?
        }
    }
    //let mut conn = Connection::new().await?;
    //let tree = conn.get_tree().await?;
    ////let windows: Vec<&Node> = tree
    ////    .iter()
    ////    .filter(|n| matches!(n.get_refined_node_type(), node_ext::RefinedNodeType::Window))
    ////    .collect();
    ////let workspaces: Vec<&Node> = tree
    ////    .iter()
    ////    .filter(|n| {
    ////        matches!(
    ////            n.get_refined_node_type(),
    ////            node_ext::RefinedNodeType::Workspace
    ////        )
    ////    })
    ////    .collect();
    ////let outputs: Vec<&Node> = tree
    ////    .iter()
    ////    .filter(|n| matches!(n.get_refined_node_type(), node_ext::RefinedNodeType::Output))
    ////    .collect();
    ////println!("windows: {:?}", windows);
    ////println!("workspaces: {:?}", workspaces);
    ////println!("outputs: {:?}", outputs);

    //utils::relayout_workspace(
    //    2,
    //    |mut conn, ws_num, ws_id, output_id, windows| async move {
    //        let master_mark = format!("_master_{}", ws_id);
    //        let main_window = windows.iter().find(|n| n.marks.contains(&master_mark));
    //        for window in windows.iter().rev() {
    //            if let Some(main_window) = main_window {
    //                if window.id == main_window.id {
    //                    continue;
    //                }
    //            }
    //            let cmd = format!(
    //                "[con_id={}] move to workspace number {}; ",
    //                window.id, ws_num
    //            );
    //            debug!("relayout closure cmd: {}", cmd);
    //            conn.run_command(cmd).await?;
    //            task::sleep(Duration::from_millis(25)).await;
    //        }
    //        if let Some(main_window) = main_window {
    //            let cmd = format!(
    //                "[con_id={}] move to workspace number {}; ",
    //                main_window.id, ws_num
    //            );
    //            debug!("relayout closure cmd: {}", cmd);
    //            conn.run_command(cmd).await?;
    //        } else {
    //            debug!("no main window found via mark: {}", master_mark);
    //        }
    //        Ok(())
    //    },
    //)
    //.await?;
    Ok(())
}
