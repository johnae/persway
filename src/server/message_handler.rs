use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use async_std::task;
use swayipc_async::{Connection, WindowEvent};

use super::command_handlers;
use super::event_handlers;

use crate::{commands::PerswayCommand, layout::WorkspaceLayout, utils};

#[derive(Debug)]
pub struct WorkspaceConfig {
    layout: WorkspaceLayout,
}

#[derive(Debug)]
pub struct MessageHandler {
    workspace_config: HashMap<i32, WorkspaceConfig>,
    default_layout: WorkspaceLayout,
    workspace_renaming: bool,
    on_window_focus: Option<String>,
    on_window_focus_leave: Option<String>,
}

impl MessageHandler {
    pub fn new(
        default_layout: WorkspaceLayout,
        workspace_renaming: bool,
        on_window_focus: Option<String>,
        on_window_focus_leave: Option<String>,
    ) -> Self {
        MessageHandler {
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
        log::debug!("controller.handle_event: {:?}", event.change);
        let mut conn = Connection::new().await?;
        let ws = utils::get_focused_workspace(&mut conn).await?;
        match &self.get_workspace_config(ws.num).layout {
            WorkspaceLayout::Spiral => {
                log::debug!("handling event via spiral manager");
                task::spawn(event_handlers::layout::spiral::Spiral::handle(
                    event.clone(),
                ));
            }
            WorkspaceLayout::StackMain { stack_layout, size } => {
                log::debug!("handling event via stack_main manager");
                task::spawn(event_handlers::layout::stack_main::StackMain::handle(
                    event.clone(),
                    *size,
                    stack_layout.clone(),
                ));
            }
            WorkspaceLayout::Manual => {}
        };
        if self.workspace_renaming {
            event_handlers::misc::workspace_renamer::WorkspaceRenamer::handle(event.clone()).await;
        }
        event_handlers::misc::window_focus::WindowFocus::handle(
            event.clone(),
            self.on_window_focus.clone(),
            self.on_window_focus_leave.clone(),
        )
        .await;
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
                        .or_insert_with(|| WorkspaceConfig {
                            layout: layout.clone(),
                        });
                    log::debug!("change layout of ws {} to {}", ws.num, layout);
                    log::debug!("start relayout of ws {}", ws.num);
                    task::spawn(utils::relayout_workspace(
                        ws.num,
                        |mut conn, ws_num, _old_ws_id, _output_id, windows| async move {
                            for window in windows.iter().rev() {
                                let cmd = format!(
                                    "[con_id={}] move to workspace number {}; [con_id={}] focus",
                                    window.id, ws_num, window.id
                                );
                                log::debug!("relayout closure cmd: {}", cmd);
                                conn.run_command(cmd).await?;
                                task::sleep(Duration::from_millis(50)).await;
                            }
                            Ok(())
                        },
                    ));
                } else {
                    log::debug!(
                        "no layout change of ws {} as the requested one was already set",
                        ws.num,
                    );
                }
            }
            PerswayCommand::StackFocusNext => match current_ws_config.layout {
                WorkspaceLayout::StackMain { .. } => {
                    let mut ctrl = command_handlers::layout::stack_main::StackMain::new().await?;
                    ctrl.stack_focus_next().await?
                }
                _ => {}
            },
            PerswayCommand::StackFocusPrev => match current_ws_config.layout {
                WorkspaceLayout::StackMain { .. } => {
                    let mut ctrl = command_handlers::layout::stack_main::StackMain::new().await?;
                    ctrl.stack_focus_prev().await?
                }
                _ => {}
            },
            PerswayCommand::StackMainRotateNext => match current_ws_config.layout {
                WorkspaceLayout::StackMain { .. } => {
                    let mut ctrl = command_handlers::layout::stack_main::StackMain::new().await?;
                    ctrl.stack_main_rotate_next().await?
                }
                _ => {}
            },
            PerswayCommand::StackSwapMain => match current_ws_config.layout {
                WorkspaceLayout::StackMain { .. } => {
                    let mut ctrl = command_handlers::layout::stack_main::StackMain::new().await?;
                    ctrl.stack_swap_main().await?
                }
                _ => {}
            },
            PerswayCommand::Daemon(_) => unreachable!(),
        }
        Ok(())
    }
}
