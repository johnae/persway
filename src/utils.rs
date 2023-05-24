use crate::node_ext::NodeExt;
use anyhow::{Context, Result};
use async_std::task;
use std::{future::Future, time::Duration};
use swayipc_async::{Connection, Node, Workspace};

pub const PERSWAY_TMP_WORKSPACE: &str = "◕‿◕";
pub const SCRATCHPAD_WORKSPACE: &str = "__i3_scratch";

pub async fn get_focused_workspace(conn: &mut Connection) -> Result<Workspace> {
    let mut ws = conn.get_workspaces().await?.into_iter();
    ws.find(|w| w.focused).context("no focused workspace")
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

pub fn is_scratchpad_workspace(ws: &Workspace) -> bool {
    ws.name == SCRATCHPAD_WORKSPACE
}

pub fn is_persway_tmp_workspace(ws: &Workspace) -> bool {
    ws.name == PERSWAY_TMP_WORKSPACE
}

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
            n.is_output()
                && n.iter()
                    .any(|n| n.is_workspace() && n.num.unwrap() == ws_num)
        })
        .context("no output found")?;
    let ws = output
        .iter()
        .find(|n| n.is_workspace() && n.num.unwrap() == ws_num)
        .context("no workspace found")?;
    let focused_workspace = workspaces
        .iter()
        .find(|w| w.focused)
        .context("no focused workspace")?;
    let mut windows: Vec<Node> = Vec::with_capacity(50);
    let mut cmd = String::from("");
    for window in ws.iter().filter(|n| n.is_window()) {
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
    task::sleep(Duration::from_millis(50)).await;
    let closure_conn = Connection::new().await?;
    f(closure_conn, ws_num, ws.id, output.id, windows).await?;
    task::sleep(Duration::from_millis(50)).await;
    let workspaces = connection.get_workspaces().await?;
    let focused_workspace_after_closure = workspaces
        .iter()
        .find(|w| w.focused)
        .context("no focused workspace")?;
    let mut cmd = String::new();
    if &focused_workspace_after_closure.num != &focused_workspace.num {
        cmd.push_str(&format!(
            "workspace number {focused_workspace_num}; move workspace to output {output_id}; ",
            focused_workspace_num = &focused_workspace.num,
            output_id = output.id,
        ));
    }
    cmd.push_str(&format!(
        "rename workspace to {ws_name}",
        ws_name = focused_workspace.name
    ));
    log::debug!(
        "rename new workspace to old name after layout closure: {}",
        cmd
    );
    connection.run_command(cmd).await?;
    Ok(())
}
