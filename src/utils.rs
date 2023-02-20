use crate::node_ext::{NodeExt, RefinedNodeType};
use anyhow::{anyhow, Context, Result};
use async_std::task;
use std::{future::Future, time::Duration};
use swayipc_async::{Connection, Node, Workspace};

const PERSWAY_TMP_WORKSPACE: &str = "◕‿◕";

pub async fn get_focused_workspace(conn: &mut Connection) -> Result<Workspace> {
    let mut ws = conn.get_workspaces().await?.into_iter();
    ws.find(|w| w.focused).context("no focused workspace")
}

pub async fn get_workspace(conn: &mut Connection, ws_num: i32) -> Result<Workspace> {
    let mut ws = conn.get_workspaces().await?.into_iter();
    ws.find(|w| w.num == ws_num)
        .context(format!("no workspace with number {}", ws_num))
}

pub fn get_socket_path(socket_path: Option<String>) -> String {
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR");
    let wayland_display = std::env::var("WAYLAND_DISPLAY");
    socket_path.unwrap_or_else(|| {
        format!(
            "{}/persway-{}.sock",
            match xdg_runtime_dir {
                Ok(dir) => dir,
                Err(_e) => {
                    log::error!("Missing XDG_RUNTIME_DIR environment variable");
                    String::from("/tmp")
                }
            },
            match wayland_display {
                Ok(path) => path,
                Err(_e) => {
                    log::error!("Missing WAYLAND_DISPLAY environment variable");
                    String::from("unknown")
                }
            }
        )
    })
}

pub fn get_main_mark(ws_id: i64) -> String {
    format!("_main_{}", ws_id)
}

pub fn get_stack_mark(ws_id: i64) -> String {
    format!("_stack_{}", ws_id)
}

//pub async fn get_workspace<'a>(ws_num: i32) -> Result<Node> {
//    let mut connection = Connection::new().await?;
//    let tree = &connection.get_tree().await?;
//    let ws = tree
//        .iter()
//        .find(|n| {
//            matches!(n.get_refined_node_type(), RefinedNodeType::Workspace)
//                && n.num.unwrap() == ws_num
//        })
//        .context("no workspace found")?;
//    let cws = ws.clone();
//    Ok(cws)
//}

pub async fn relayout_workspace<F, C>(ws_num: i32, f: C) -> Result<()>
where
    F: Future<Output = Result<()>>,
    C: FnOnce(Connection, i32, i64, i64, Vec<Node>) -> F,
{
    let mut connection = Connection::new().await?;
    let tree = connection.get_tree().await?;
    let workspaces = connection.get_workspaces().await?;
    let output = tree
        .iter()
        .find(|n| {
            matches!(n.get_refined_node_type(), RefinedNodeType::Output)
                && n.iter().any(|n| {
                    matches!(n.get_refined_node_type(), RefinedNodeType::Workspace)
                        && n.num.unwrap() == ws_num
                })
        })
        .context("no output found")?;
    let ws = output
        .iter()
        .find(|n| {
            matches!(n.get_refined_node_type(), RefinedNodeType::Workspace)
                && n.num.unwrap() == ws_num
        })
        .context("no workspace found")?;
    //let output_visible_workspace = workspaces
    //    .iter()
    //    .find(|w| w.visible)
    //    .context(format!("no visible workspace on output: {}", output.id))?;
    let focused_workspace = workspaces
        .iter()
        .find(|w| w.focused)
        .context("no focused workspace")?;
    let mut windows: Vec<Node> = Vec::with_capacity(50);
    let mut cmd = String::from("");
    for window in ws
        .iter()
        .filter(|n| matches!(n.get_refined_node_type(), RefinedNodeType::Window))
    {
        windows.push(window.clone());
        cmd.push_str(&format!(
            "[con_id={}] move to workspace {}; ",
            window.id, PERSWAY_TMP_WORKSPACE
        ));
    }
    cmd.push_str(&format!(
        "workspace {}; move workspace to output {}; ",
        PERSWAY_TMP_WORKSPACE, output.id
    ));
    log::debug!("relayout before layout closure: {}", cmd);
    connection.run_command(cmd).await?;
    task::sleep(Duration::from_millis(25)).await;
    //let cmd = format!(
    //    "workspace number {}; move workspace to output {}; ",
    //    &focused_workspace.num, output.id
    //);
    //log::debug!("relayout after layout closure: {}", cmd);
    //connection.run_command(cmd).await?;
    let closure_conn = Connection::new().await?;
    f(closure_conn, ws_num, ws.id, output.id, windows).await?;
    task::sleep(Duration::from_millis(25)).await;
    let workspaces = connection.get_workspaces().await?;
    let focused_workspace_after_closure = workspaces
        .iter()
        .find(|w| w.focused)
        .context("no focused workspace")?;
    if &focused_workspace_after_closure.num != &focused_workspace.num {
        let cmd = format!(
            "workspace number {}; move workspace to output {}; ",
            &focused_workspace.num, output.id
        );
        log::debug!("relayout after layout closure: {}", cmd);
        connection.run_command(cmd).await?;
    } else {
        log::debug!("skip relayout after layout closure");
    }
    Ok(())
}
