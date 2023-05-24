use crate::{
    layout::StackLayout,
    node_ext::NodeExt,
    utils::{get_focused_workspace, is_persway_tmp_workspace, is_scratchpad_workspace},
};

use anyhow::Result;
use async_trait::async_trait;
use swayipc_async::{Connection, WindowChange, WindowEvent, Workspace};

use super::super::traits::WindowEventHandler;

fn should_skip_layout_of_workspace(workspace: &Workspace) -> bool {
    is_persway_tmp_workspace(workspace) || is_scratchpad_workspace(workspace)
}

pub struct StackMain {
    connection: Connection,
    size: u8,
    stack_layout: StackLayout,
}

impl StackMain {
    pub async fn handle(event: Box<WindowEvent>, size: u8, stack_layout: StackLayout) {
        if let Ok(mut manager) = Self::new(size, stack_layout).await {
            manager.handle(event).await;
        }
    }

    pub async fn new(size: u8, stack_layout: StackLayout) -> Result<Self> {
        let connection = Connection::new().await?;
        Ok(Self {
            connection,
            size,
            stack_layout,
        })
    }

    async fn on_new_window(&mut self, event: &WindowEvent) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let node = tree
            .find_as_ref(|n| n.id == event.container.id)
            .expect(&format!("no node found with id {}", event.container.id));
        let ws = node.get_workspace().await?;
        if should_skip_layout_of_workspace(&ws) {
            log::debug!("skip stack_main layout of \"special\" workspace");
            return Ok(());
        }
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        log::debug!("new_window id: {}", event.container.id);
        log::debug!("workspace nodes len: {}", wstree.nodes.len());
        let layout = match self.stack_layout {
            StackLayout::Tabbed => "split v; layout tabbed",
            StackLayout::Stacked => "split v; layout stacking",
            StackLayout::Tiled => "split v",
        };
        match wstree.nodes.len() {
            1 => {
                let cmd = format!("[con_id={}] focus; split h", event.container.id);
                self.connection.run_command(cmd).await?;
                Ok(())
            }
            2 => {
                let main = wstree.nodes.last().expect("main window not found");
                let stack = wstree.nodes.first().expect("stack container not found");

                let cmd = if stack.is_window() {
                    format!(
                        "[con_id={}] focus; {}; resize set width {}; [con_id={}] focus",
                        stack.id,
                        layout,
                        (100 - self.size),
                        main.id
                    )
                } else {
                    if let Some(node) = stack.find_as_ref(|n| n.id == event.container.id) {
                        format!(
                            "[con_id={}] focus; swap container with con_id {}; [con_id={}] focus",
                            main.id, node.id, node.id
                        )
                    } else {
                        String::from("nop event container not in stack")
                    }
                };

                self.connection.run_command(cmd).await?;
                Ok(())
            }
            3 => {
                let main = wstree
                    .nodes
                    .iter()
                    .skip(1)
                    .find(|n| n.is_window() && n.id != event.container.id)
                    .expect("main window not found");
                let stack = wstree.nodes.first().expect("stack container not found");
                let stack_mark = format!("_stack_{}", stack.id);

                let cmd = format!(
                    "[con_id={}] mark --add {}; [con_id={}] focus; move container to mark {}; [con_mark={}] unmark {}; [con_id={}] focus; swap container with con_id {}; [con_id={}] focus",
                          stack.id, stack_mark,
                          event.container.id, stack_mark,
                          stack_mark, stack_mark,
                          main.id, event.container.id, event.container.id
                );

                log::debug!("new_window: {}", cmd);

                self.connection.run_command(cmd).await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
    async fn on_close_window(&mut self, event: &WindowEvent) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        if should_skip_layout_of_workspace(&ws) {
            log::debug!("skip stack_main layout of \"special\" workspace");
            return Ok(());
        }

        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();

        if wstree.nodes.len() == 1 {
            if let Some(stack) = wstree
                .nodes
                .iter()
                .filter(|n| n.id != event.container.id)
                .next()
            {
                let stack_current = stack
                    .find_as_ref(|n| n.is_window() && n.focused)
                    .unwrap_or_else(|| {
                        stack
                            .find_as_ref(|n| n.visible.unwrap_or(false))
                            .expect("stack should have a visible node")
                    });

                let cmd = if wstree.iter().filter(|n| n.is_window()).count() == 1 {
                    log::debug!("on_close_window, count 1, stack_id: {}", stack_current.id);
                    format!(
                        "[con_id={}] focus; layout splith; move up",
                        stack_current.id
                    )
                } else {
                    log::debug!(
                        "on_close_window, count more than 1, stack_id: {}",
                        stack_current.id
                    );
                    format!(
                        "[con_id={}] focus; move right; resize set width {}",
                        stack_current.id, self.size
                    )
                };
                log::debug!("close_window: {}", cmd);
                self.connection.run_command(cmd).await?;
            }
        }
        Ok(())
    }
    async fn on_move_window(&mut self, event: &WindowEvent) -> Result<()> {
        let tree = self.connection.get_tree().await?;

        let node = if let Some(node) = tree.find_as_ref(|n| n.id == event.container.id) {
            node
        } else {
            log::warn!("no node found with id {}", event.container.id);
            return Ok(());
        };

        let ws = if let Ok(ws) = node.get_workspace().await {
            ws
        } else {
            log::warn!("node had no workspace");
            return self.on_close_window(event).await;
        };

        if should_skip_layout_of_workspace(&ws) {
            log::debug!("skip stack_main layout of \"special\" workspace");
            return Ok(());
        }

        let focused_ws = get_focused_workspace(&mut self.connection).await?;

        if ws.id == focused_ws.id {
            log::debug!("move_window within workspace: {}", ws.num);
            return self.on_new_window(event).await;
        }

        log::debug!("move_window to other workspace: {}", ws.num);
        self.on_new_window(event).await?;
        self.on_close_window(event).await
    }
}
#[async_trait]
impl WindowEventHandler for StackMain {
    async fn handle(&mut self, event: Box<WindowEvent>) {
        match event.change {
            WindowChange::New => {
                log::debug!("stack_main handler handling event: {:?}", event.change);
                if let Err(e) = self.on_new_window(&event).await {
                    log::error!("stack_main layout err: {}", e);
                };
            }
            WindowChange::Close => {
                log::debug!("stack_main handler handling event: {:?}", event.change);
                if let Err(e) = self.on_close_window(&event).await {
                    log::error!("stack_main layout err: {}", e);
                };
            }
            WindowChange::Move => {
                log::debug!("stack_main handler handling event: {:?}", event.change);
                if let Err(e) = self.on_move_window(&event).await {
                    log::error!("stack_main layout err: {}", e);
                };
            }
            _ => {
                log::debug!("stack_main not handling event: {:?}", event.change);
            }
        }
    }
}
