use crate::{
    node_ext::NodeExt,
    utils::{get_focused_workspace, get_main_mark, get_stack_mark},
};
use anyhow::Result;
use swayipc_async::Connection;

pub struct StackMain {
    connection: Connection,
}

impl StackMain {
    pub async fn new() -> Result<Self> {
        let connection = Connection::new().await?;
        Ok(Self { connection })
    }

    pub async fn stack_focus_prev(&mut self) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let stack_mark = get_stack_mark(ws.id);
        let main_mark = get_main_mark(ws.id);

        if let Some(stack) = wstree.find_as_ref(|n| n.marks.contains(&stack_mark)) {
            if stack.nodes.len() == 0 {
                return Ok(());
            }
            let stack_visible = stack
                .find_as_ref(|n| n.is_window() && n.visible.unwrap_or(false))
                .expect("stack contains no visible node");
            let mut prev_was_visible = false;
            for node in stack.nodes.iter().rev().cycle() {
                if prev_was_visible {
                    let cmd = format!("[con_id={}] focus; [con_mark={}] focus", node.id, main_mark);
                    log::debug!("stack main controller, stack focus prev: {}", cmd);
                    self.connection.run_command(cmd).await?;
                    return Ok(());
                }
                prev_was_visible = node.id == stack_visible.id
            }
        }
        Ok(())
    }

    pub async fn stack_focus_next(&mut self) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let stack_mark = get_stack_mark(ws.id);
        let main_mark = get_main_mark(ws.id);

        if let Some(stack) = wstree.find_as_ref(|n| n.marks.contains(&stack_mark)) {
            if stack.nodes.len() == 0 {
                return Ok(());
            }
            let stack_visible = stack
                .find_as_ref(|n| n.is_window() && n.visible.unwrap_or(false))
                .expect("stack contains no visible node");
            let mut prev_was_visible = false;
            for node in stack.nodes.iter().cycle() {
                if prev_was_visible {
                    let cmd = format!("[con_id={}] focus; [con_mark={}] focus", node.id, main_mark);
                    log::debug!("stack main controller, stack focus next: {}", cmd);
                    self.connection.run_command(cmd).await?;
                    return Ok(());
                }
                prev_was_visible = node.id == stack_visible.id
            }
        }
        Ok(())
    }

    pub async fn stack_main_rotate_next(&mut self) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let stack_mark = get_stack_mark(ws.id);
        let main_mark = get_main_mark(ws.id);

        if let Some(stack) = wstree.find_as_ref(|n| n.marks.contains(&stack_mark)) {
            if stack.nodes.len() == 0 {
                return Ok(());
            }

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
                "[con_id={}] focus; [con_mark={}] focus; ",
                stack.nodes.last().unwrap().id,
                main_mark
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
                "[con_id={}] focus; swap container with con_id {}; [con_id={}] focus; [con_mark={}] unmark {}; [con_id={}] mark --add {}",
                main.id, stack_first, stack_first,
                main_mark, main_mark, stack_first, main_mark
            );
            log::debug!("stack main controller, master cycle next 2: {}", cmd);
            self.connection.run_command(cmd).await?;
            return Ok(());
        }
        Ok(())
    }

    pub async fn swap_visible(&mut self) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let stack_mark = get_stack_mark(ws.id);
        let main_mark = get_main_mark(ws.id);

        if let Some(stack) = wstree.find_as_ref(|n| n.marks.contains(&stack_mark)) {
            if stack.nodes.len() == 0 {
                return Ok(());
            }

            let main = wstree.nodes.last().expect("main window not found");
            let stack_visible = stack
                .find_as_ref(|n| n.is_window() && n.visible.unwrap_or(false))
                .expect("stack contains no visible node");
            let cmd = format!(
                "[con_id={}] focus; swap container with con_id {}; [con_id={}] focus; [con_mark={}] unmark {}; [con_id={}] mark --add {}",
                main.id, stack_visible.id, stack_visible.id, main_mark, main_mark, stack_visible.id, main_mark
            );
            log::debug!("stack main controller, swap visible: {}", cmd);
            self.connection.run_command(cmd).await?;
        }
        Ok(())
    }
}
