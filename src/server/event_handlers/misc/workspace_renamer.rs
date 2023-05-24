use super::super::traits::WindowEventHandler;
use crate::utils;

use anyhow::Result;
use async_trait::async_trait;
use swayipc_async::{Connection, WindowChange, WindowEvent, Workspace};

pub struct WorkspaceRenamer {
    connection: Connection,
}

fn should_skip_rename_of_workspace(workspace: &Workspace) -> bool {
    utils::is_persway_tmp_workspace(workspace) || utils::is_scratchpad_workspace(workspace)
}

fn get_app_name(event: &WindowEvent) -> Option<String> {
    let app_id =
        event
            .container
            .app_id
            .as_ref()
            .and_then(|id| if id.is_empty() { None } else { Some(id) });

    let name = event.container.name.as_ref().and_then(|name| {
        if name.is_empty() {
            None
        } else {
            name.split('|').next().map(|s| s.to_string())
        }
    });

    let class = event.container.window_properties.as_ref().and_then(|p| {
        p.class
            .as_ref()
            .and_then(|class| if class.is_empty() { None } else { Some(class) })
    });

    let app_name = app_id.or(class);
    let app_name = app_name.or(name.as_ref());
    app_name.map(|n| {
        n.trim_start_matches('-')
            .trim_end_matches('-')
            .trim_end_matches(' ')
            .to_lowercase()
    })
}

impl WorkspaceRenamer {
    pub async fn handle(event: Box<WindowEvent>) {
        if let Ok(mut manager) = Self::new().await {
            manager.handle(event).await;
        }
    }

    pub async fn new() -> Result<Self> {
        let connection = Connection::new().await?;
        Ok(Self { connection })
    }

    async fn rename_workspace(&mut self, event: WindowEvent) -> Result<()> {
        log::debug!("workspace name manager handling event: {:?}", event.change);
        let focused_ws = utils::get_focused_workspace(&mut self.connection).await?;
        if should_skip_rename_of_workspace(&focused_ws) {
            log::debug!("workspace name manager skip renaming workspace");
            return Ok(());
        }

        let ws_num = focused_ws
            .name
            .split(":")
            .next()
            .unwrap_or(&focused_ws.name);
        if let Some(app_name) = get_app_name(&event) {
            let cmd = format!("rename workspace to {}: {}", ws_num, app_name);
            log::debug!("workspace name manager, cmd: {}", cmd);
            self.connection.run_command(cmd).await?;
        } else {
            log::error!("workspace name manager failed to set a workspace name");
        }

        Ok(())
    }
}
#[async_trait]
impl WindowEventHandler for WorkspaceRenamer {
    async fn handle(&mut self, event: Box<WindowEvent>) {
        match event.change {
            WindowChange::Focus => {
                if let Err(e) = self.rename_workspace(*event).await {
                    log::error!("workspace name manager, layout err: {}", e);
                };
            }
            WindowChange::Close => {
                if let Err(e) = self.rename_workspace(*event).await {
                    log::error!("workspace name manager, layout err: {}", e);
                };
            }
            _ => log::debug!(
                "workspace name manager, not handling event: {:?}",
                event.change
            ),
        }
    }
}
