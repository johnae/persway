use std::{collections::HashMap, ops::Deref};

use anyhow::Result;
use swayipc_async::{Connection, WindowEvent};

use super::managers::{self, traits::WindowEventHandler};

use crate::{
    commands::PerswayCommand,
    layout::{self, WorkspaceLayout},
    utils,
};

#[derive(Debug)]
struct WorkspaceConfig {
    layout: WorkspaceLayout,
}

#[derive(Debug)]
pub struct Controller {
    workspace_config: HashMap<i32, WorkspaceConfig>,
    default_layout: WorkspaceLayout,
    workspace_renaming: bool,
    on_window_focus: Option<String>,
    on_window_focus_leave: Option<String>,
}

impl Controller {
    pub fn new(
        default_layout: WorkspaceLayout,
        workspace_renaming: bool,
        on_window_focus: Option<String>,
        on_window_focus_leave: Option<String>,
    ) -> Self {
        Controller {
            workspace_config: HashMap::new(),
            default_layout,
            workspace_renaming,
            on_window_focus,
            on_window_focus_leave,
        }
    }

    pub fn get_workspace_config(&mut self, ws_num: i32) -> &WorkspaceConfig {
        self.workspace_config
            .entry(ws_num)
            .or_insert_with(|| WorkspaceConfig {
                layout: self.default_layout.clone(),
            })
    }

    pub async fn handle_event(&mut self, event: Box<WindowEvent>) -> Result<()> {
        log::debug!("controller.handle_event: {:?}", event);
        let mut conn = Connection::new().await?;
        let ws = utils::get_focused_workspace(&mut conn).await?;
        match self.get_workspace_config(ws.num).layout {
            WorkspaceLayout::Spiral => {}
            WorkspaceLayout::StackMain => {
                log::debug!("handling event via stack_main manager");
                let mut manager = managers::layout::stack_main::StackMain::new().await?;
                manager.handle(&event).await;
            }
            WorkspaceLayout::Manual => {}
        };
        Ok(())
    }
    pub async fn handle_command(&mut self, cmd: PerswayCommand) -> Result<()> {
        log::debug!("controller.handle_command: {:?}", cmd);
        let mut conn = Connection::new().await?;
        let ws = utils::get_focused_workspace(&mut conn).await?;
        let current_ws_config = self.get_workspace_config(ws.num);
        match cmd {
            PerswayCommand::ChangeLayout { layout } => {
                if current_ws_config.layout != layout {
                    self.workspace_config
                        .entry(ws.num)
                        .and_modify(|e| e.layout = layout.clone())
                        .or_insert_with(|| WorkspaceConfig { layout });
                    log::debug!("change layout of ws {}: {:?}", ws.num, self);
                } else {
                    log::debug!(
                        "no layout change of ws {} as the requested one was already set",
                        ws.num,
                    );
                }
            }
            PerswayCommand::StackFocusNext => {
                if current_ws_config.layout == WorkspaceLayout::StackMain {}
            }
            PerswayCommand::StackFocusPrev => {
                if current_ws_config.layout == WorkspaceLayout::StackMain {}
            }
            PerswayCommand::StackMainRotateNext => {
                if current_ws_config.layout == WorkspaceLayout::StackMain {}
            }
            PerswayCommand::StackMainRotatePrev => {
                if current_ws_config.layout == WorkspaceLayout::StackMain {}
            }
            PerswayCommand::StackSwapVisible => {
                if current_ws_config.layout == WorkspaceLayout::StackMain {}
            }
            PerswayCommand::Daemon(_) => unreachable!(),
        }
        Ok(())
    }
}
