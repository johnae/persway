use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use async_std::task;
use swayipc_async::{Connection, WindowEvent};

use super::managers::{self, traits::WindowEventHandler};

use crate::{
    commands::PerswayCommand,
    layout::WorkspaceLayout,
    utils::{self, get_main_mark},
};

#[derive(Debug)]
pub struct WorkspaceConfig {
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

    async fn spiral_handler(event: Box<WindowEvent>) {
        if let Ok(mut manager) = managers::layout::spiral::Spiral::new().await {
            manager.handle(event).await;
        }
    }

    async fn stack_main_handler(event: Box<WindowEvent>) {
        if let Ok(mut manager) = managers::layout::stack_main::StackMain::new().await {
            manager.handle(event).await;
        }
    }

    pub async fn handle_event(&mut self, event: Box<WindowEvent>) -> Result<()> {
        log::debug!("controller.handle_event: {:?}", event.change);
        let mut conn = Connection::new().await?;
        let ws = utils::get_focused_workspace(&mut conn).await?;
        match self.get_workspace_config(ws.num).layout {
            WorkspaceLayout::Spiral => {
                log::debug!("handling event via spiral manager");
                task::spawn(Self::spiral_handler(event));
            }
            WorkspaceLayout::StackMain => {
                log::debug!("handling event via stack_main manager");
                task::spawn(Self::stack_main_handler(event));
                //task::spawn(|| async {
                //    let mut manager = managers::layout::stack_main::StackMain::new().await?;
                //    manager.handle(&event).await;
                //});
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
                        .or_insert_with(|| WorkspaceConfig {
                            layout: layout.clone(),
                        });
                    log::debug!("change layout of ws {} to {}", ws.num, layout);
                    log::debug!("start relayout of ws {}", ws.num);
                    task::spawn(utils::relayout_workspace(
                        ws.num,
                        |mut conn, ws_num, old_ws_id, _output_id, windows| async move {
                            let main_mark = get_main_mark(old_ws_id);
                            let main_window = windows.iter().find(|n| n.marks.contains(&main_mark));
                            for window in windows.iter().rev() {
                                if let Some(main_window) = main_window {
                                    if window.id == main_window.id {
                                        continue;
                                    }
                                }
                                let cmd = format!(
                                    "[con_id={}] move to workspace number {}; [con_id={}] focus",
                                    window.id, ws_num, window.id
                                );
                                log::debug!("relayout closure cmd: {}", cmd);
                                conn.run_command(cmd).await?;
                                task::sleep(Duration::from_millis(25)).await;
                            }
                            if let Some(main_window) = main_window {
                                let cmd = format!(
                                    "[con_id={}] move to workspace number {}; [con_id={}] focus",
                                    main_window.id, ws_num, main_window.id
                                );
                                log::debug!("relayout closure cmd: {}", cmd);
                                conn.run_command(cmd).await?;
                            } else {
                                log::debug!("no main window found via mark: {}", main_mark);
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
