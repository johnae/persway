use crate::{node_ext::NodeExt, utils::get_focused_workspace};
use anyhow::Result;
use either::Either;
use swayipc_async::Connection;

pub struct StackMain {
    connection: Connection,
}

impl StackMain {
    pub async fn new() -> Result<Self> {
        let connection = Connection::new().await?;
        Ok(Self { connection })
    }

    async fn stack_focus_advance(&mut self, reverse: bool) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();

        if let Some(stack) = wstree.nodes.first() {
            if stack.nodes.len() == 0 {
                return Ok(());
            }

            let focused = stack.find_as_ref(|n| n.is_window() && n.focused);
            let visible = stack
                .iter()
                .filter(|n| n.is_window() && n.visible.unwrap_or(false));
            let initial = if reverse {
                stack.nodes.first()
            } else {
                stack.nodes.last()
            };

            let stack_current = if let Some(focused) = focused {
                focused
            } else if visible.count() == 1 {
                stack.find_as_ref(|n| n.visible.unwrap_or(false)).unwrap()
            } else {
                initial.unwrap()
            };

            let mut prev_was_focused = false;
            let stack_iter = match reverse {
                true => Either::Left(stack.nodes.iter().rev()),
                false => Either::Right(stack.nodes.iter()),
            };

            for node in stack_iter.cycle() {
                if prev_was_focused {
                    let cmd = format!("[con_id={}] focus;", node.id);
                    log::debug!("stack main controller, stack focus prev: {}", cmd);
                    self.connection.run_command(cmd).await?;
                    return Ok(());
                }
                prev_was_focused = node.id == stack_current.id
            }
        }
        Ok(())
    }

    pub async fn stack_focus_prev(&mut self) -> Result<()> {
        self.stack_focus_advance(true).await
    }

    pub async fn stack_focus_next(&mut self) -> Result<()> {
        self.stack_focus_advance(false).await
    }

    pub async fn stack_main_rotate_next(&mut self) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();

        if let Some(stack) = wstree.nodes.first() {
            if stack.nodes.len() == 0 {
                return Ok(());
            }

            let main = wstree.nodes.last().expect("main window not found");
            let stack_leaves = stack.iter().filter(|n| n.is_window());

            let mut stack_leaves_next = stack_leaves.clone();
            stack_leaves_next.next();

            let mut cmd = String::from("");
            for node in stack_leaves {
                if let Some(next) = stack_leaves_next.next() {
                    cmd.push_str(&format!(
                        "[con_id={}] focus; swap container with con_id {}; ",
                        node.id, next.id
                    ));
                } else {
                    break;
                }
            }
            cmd.push_str(&format!(
                "[con_id={}] focus; [con_id={}] focus; ",
                stack.nodes.last().unwrap().id,
                main.id
            ));
            log::debug!("stack main controller, master cycle next 1: {}", cmd);
            self.connection.run_command(cmd).await?;

            let tree = self.connection.get_tree().await?;
            let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
            let main = wstree.nodes.last().expect("main window not found");
            let stack = wstree.nodes.first().expect("stack container not found");

            let stack_first = stack
                .iter()
                .filter(|n| n.is_window())
                .map(|n| n.id)
                .collect::<Vec<_>>()
                .into_iter()
                .next()
                .unwrap();

            let cmd = format!(
                "[con_id={}] focus; swap container with con_id {}; [con_id={}] focus",
                main.id, stack_first, stack_first,
            );
            log::debug!("stack main controller, master cycle next 2: {}", cmd);
            self.connection.run_command(cmd).await?;
            return Ok(());
        }
        Ok(())
    }

    pub async fn stack_swap_main(&mut self) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();

        if let Some(stack) = wstree.nodes.first() {
            if stack.nodes.len() == 0 {
                return Ok(());
            }

            let main = wstree.nodes.last().expect("main window not found");

            let focused = stack.find_as_ref(|n| n.is_window() && n.focused);
            let visible = stack
                .iter()
                .filter(|n| n.is_window() && n.visible.unwrap_or(false));
            let initial = stack.nodes.first();

            let stack_current = if let Some(focused) = focused {
                focused
            } else if visible.count() == 1 {
                stack.find_as_ref(|n| n.visible.unwrap_or(false)).unwrap()
            } else {
                initial.unwrap()
            };

            let cmd = format!(
                "[con_id={}] focus; swap container with con_id {}; [con_id={}] focus",
                main.id, stack_current.id, stack_current.id
            );
            log::debug!("stack main controller, swap visible: {}", cmd);
            self.connection.run_command(cmd).await?;
        }
        Ok(())
    }
}
