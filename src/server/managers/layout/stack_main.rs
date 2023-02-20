use crate::{
    node_ext::{NodeExt, RefinedNodeType},
    utils::{get_focused_workspace, get_main_mark, get_stack_mark},
};
use std::ops::RangeBounds;

use anyhow::Result;
use async_trait::async_trait;
use swayipc_async::{Connection, WindowChange, WindowEvent};

use super::super::traits::WindowEventHandler;

pub struct StackMain {
    connection: Connection,
}

impl StackMain {
    pub async fn new() -> Result<Self> {
        let connection = Connection::new().await?;
        Ok(Self { connection })
    }

    async fn on_new_window(
        &mut self,
        event: &WindowEvent,
    ) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let node = tree
            .find_as_ref(|n| n.id == event.container.id)
            .expect(&format!("no node found with id {}", event.container.id));
        let ws = node.get_workspace().await?;
        //let ws = get_node_workspace(&mut self.connection, event.container.id).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        log::debug!("new_window: {:?}", event.container.id);
        log::debug!("nodes_len: {}", wstree.nodes.len());
        match wstree.nodes.len() {
            1 => {
                let main_mark = get_main_mark(ws.id);
                let cmd = format!(
                    "[con_mark={}] unmark {}; split h; [con_id={}] mark --add {}",
                    main_mark, main_mark, event.container.id, main_mark
                );
                Ok(self.connection.run_command(cmd).await?)
            }
            2 => {
                let stack_mark = get_stack_mark(ws.id);
                let main_mark = get_main_mark(ws.id);
                let main = wstree.nodes.last().expect("main window not found");
                let stack = wstree.nodes.first().expect("stack container not found");

                let cmd = if stack.is_window() {
                    format!(
                      "[con_id={}] mark --add {}; [con_id={}] focus; split v; layout stacking; resize set width 25; [con_id={}] focus parent; mark --add {}; [con_id={}] focus",
                      main.id, main_mark, stack.id, stack.id, stack_mark, main.id
                    )
                } else {
                    format!(
                        "[con_id={}] mark --add {}; [con_id={}] mark --add {}",
                        stack.id, stack_mark, main.id, main_mark
                    )
                };

                Ok(self.connection.run_command(cmd).await?)
            }
            3 => {
                let main = wstree
                    .nodes
                    .iter()
                    .skip(1)
                    .find(|n| n.is_window() && n.id != event.container.id)
                    .expect("main window not found");
                let stack = wstree.nodes.first().expect("stack container not found");

                let stack_mark = get_stack_mark(ws.id);
                let main_mark = get_main_mark(ws.id);

                let cmd = format!(
                          "[con_mark={}] unmark {}; [con_mark={}] unmark {}; [con_id={}] mark --add {}; [con_id={}] mark --add {}; [con_id={}] focus; move container to mark {}; [con_id={}] focus; swap container with con_id {}; [con_id={}] focus",
                          stack_mark, stack_mark, main_mark, main_mark, stack.id, stack_mark, event.container.id, main_mark,
                          event.container.id, stack_mark,
                          main.id, event.container.id, event.container.id
                );

                log::debug!("new_window: {}", cmd);

                Ok(self.connection.run_command(cmd).await?)
            }
            _ => Ok(Vec::new()),
        }
    }
    async fn on_close_window(
        &mut self,
        event: &WindowEvent,
    ) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let main_mark = get_main_mark(ws.id);

        if event.container.marks.contains(&main_mark) {
            if let Some(stack) = wstree
                .nodes
                .iter()
                .filter(|n| n.id != event.container.id)
                .next()
            {
                let stack_visible = stack
                    .find_as_ref(|n| n.visible.unwrap_or(false))
                    .expect("no visible stack node");
                let cmd = if wstree.iter().filter(|n| n.is_window()).count() == 1 {
                    log::debug!("count is 1..., stack_visible: {:?}", stack_visible);
                    format!(
                        "[con_id={}] focus; layout splith; move up; [con_id={}] mark --add {}",
                        stack_visible.id, stack_visible.id, main_mark
                    )
                } else {
                    log::debug!("count is more than 1...");
                    format!(
                      "[con_id={}] focus; move right; resize set width 75; [con_id={}] mark --add {}",
                      stack_visible.id, stack_visible.id, main_mark
                    )
                };
                log::debug!("close_window: {}", cmd);
                return Ok(self.connection.run_command(cmd).await?);
            }
        }
        Ok(Vec::new())
    }
    async fn on_move_window(
        &mut self,
        event: &WindowEvent,
    ) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let node = tree
            .find_as_ref(|n| n.id == event.container.id)
            .expect(&format!("no node found with id {}", event.container.id));
        let ws = node.get_workspace().await?;
        let focused_ws = get_focused_workspace(&mut self.connection).await?;

        if ws.id == focused_ws.id {
            log::debug!("move_window within workspace: {:?}", event);
            return self.on_new_window(event).await;
        }

        log::debug!("move_window to other workspace: {:?}", event);
        self.on_new_window(event).await?;
        self.on_close_window(event).await
    }
}
#[async_trait]
impl WindowEventHandler for StackMain {
    async fn handle(&mut self, event: &Box<WindowEvent>) {
        match event.change {
            WindowChange::New => {
                if let Err(e) = self.on_new_window(&event).await {
                    log::error!("master/stack layout err: {}", e);
                };
            }
            WindowChange::Close => {
                if let Err(e) = self.on_close_window(&event).await {
                    log::error!("master/stack layout err: {}", e);
                };
            }
            WindowChange::Move => {
                if let Err(e) = self.on_move_window(&event).await {
                    log::error!("master/stack layout err: {}", e);
                };
            }
            _ => {
                log::debug!("MasterStackHandler, not handling event: {:?}", event.change);
            }
        }
    }
}
