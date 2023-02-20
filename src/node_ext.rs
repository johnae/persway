use anyhow::{anyhow, Result};
use async_trait::async_trait;
use swayipc_async::{Connection, Node, NodeLayout, NodeType, Workspace};

pub const WS_PREFIX: &str = "ws_";
pub const CON_PREFIX: &str = "con_";
pub const MASTER_PREFIX: &str = "master_";
pub const STACK_PREFIX: &str = "stack_";

pub enum RefinedNodeType {
    Root,
    Output,
    Workspace,
    Container,         // doesn't directly contain an application
    FloatingContainer, // doesn't directly contain an application
    FloatingWindow,    // directly contains an application
    Window,            // directly contains an application
}

#[derive(Clone)]
pub struct LinearNodeIterator<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> LinearNodeIterator<'a> {
    fn new(root: &'a Node) -> LinearNodeIterator<'a> {
        let mut stack = Vec::with_capacity(100);
        stack.push(root);
        LinearNodeIterator { stack }
    }
}

impl<'a> Iterator for LinearNodeIterator<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        for entry in &node.floating_nodes {
            self.stack.push(entry);
        }
        for entry in &node.nodes {
            self.stack.push(entry);
        }
        Some(node)
    }
}

#[async_trait]
pub trait NodeExt {
    //fn get_workspace_num_from_mark(&self) -> Result<i32>;
    //async fn get_current_workspace_num(&self) -> Result<i32>;

    async fn get_workspace(&self) -> Result<Workspace>;
    fn get_refined_node_type(&self) -> RefinedNodeType;
    fn iter(&self) -> LinearNodeIterator;
    fn is_root(&self) -> bool;
    fn is_output(&self) -> bool;
    fn is_workspace(&self) -> bool;
    fn is_container(&self) -> bool;
    fn is_floating_container(&self) -> bool;
    fn is_window(&self) -> bool;
    fn is_floating_window(&self) -> bool;
    //fn get_app_name(&self) -> &str;
    //fn is_floating(&self) -> bool;
    //fn is_full_screen(&self) -> bool;
    //fn is_stacked(&self) -> bool;
    //fn is_tabbed(&self) -> bool;
}

#[async_trait]
impl NodeExt for Node {
    fn iter(&self) -> LinearNodeIterator {
        LinearNodeIterator::new(self)
    }

    //    fn get_workspace_num_from_mark(&self) -> Result<i32> {
    //        let mark = self
    //            .marks
    //            .iter()
    //            .filter(|m| m.starts_with(WS_PREFIX))
    //            .next()
    //            .context("finding ws mark on node")?;
    //        Ok(mark
    //            .split("+")
    //            .next()
    //            .expect("parsing out ws mark part")
    //            .split("_")
    //            .last()
    //            .expect("parsing ws mark")
    //            .parse()
    //            .context("parse ws mark to number")?)
    //    }

    //async fn get_current_workspace_num(&self) -> Result<i32> {
    //    let mut connection = Connection::new().await?;
    //    let tree = connection.get_tree().await?;
    //    if let Some(wstree) = tree.find_as_ref(|n| {
    //        n.node_type == NodeType::Workspace && n.find_as_ref(|n| n.id == self.id).is_some()
    //    }) {
    //        return Ok(wstree.num.unwrap());
    //    }
    //    Err(anyhow!("can't find the current node workspace"))
    //}

    async fn get_workspace(&self) -> Result<Workspace> {
        let mut connection = Connection::new().await?;
        let tree = connection.get_tree().await?;
        let workspaces = connection.get_workspaces().await?;
        let wsnode = tree
            .find(|n| {
                matches!(n.get_refined_node_type(), RefinedNodeType::Workspace)
                    && n.iter().any(|n| n.id == self.id)
            })
            .ok_or(anyhow!(format!(
                "no workspace found for node with id {}",
                self.id
            )))?;
        workspaces
            .iter()
            .find(|w| w.id == wsnode.id)
            .ok_or(anyhow!(format!(
                "hmm no workspace found with id {}",
                wsnode.id
            )))
            .cloned()
    }

    fn is_root(&self) -> bool {
        matches!(self.get_refined_node_type(), RefinedNodeType::Root)
    }
    fn is_output(&self) -> bool {
        matches!(self.get_refined_node_type(), RefinedNodeType::Output)
    }
    fn is_workspace(&self) -> bool {
        matches!(self.get_refined_node_type(), RefinedNodeType::Workspace)
    }
    fn is_container(&self) -> bool {
        matches!(self.get_refined_node_type(), RefinedNodeType::Container)
    }
    fn is_floating_container(&self) -> bool {
        matches!(
            self.get_refined_node_type(),
            RefinedNodeType::FloatingContainer
        )
    }
    fn is_window(&self) -> bool {
        matches!(self.get_refined_node_type(), RefinedNodeType::Window)
    }
    fn is_floating_window(&self) -> bool {
        matches!(
            self.get_refined_node_type(),
            RefinedNodeType::FloatingWindow
        )
    }

    fn get_refined_node_type(&self) -> RefinedNodeType {
        match self.node_type {
            NodeType::Root => RefinedNodeType::Root,
            NodeType::Output => RefinedNodeType::Output,
            NodeType::Workspace => RefinedNodeType::Workspace,
            _ => {
                if self.node_type == NodeType::Con
                    && self.name.is_none()
                    && self.app_id.is_none()
                    && self.pid.is_none()
                    && self.shell.is_none()
                    && self.window_properties.is_none()
                    && self.layout != NodeLayout::None
                {
                    RefinedNodeType::Container
                } else if self.node_type == NodeType::FloatingCon
                    && self.name.is_none()
                    && self.app_id.is_none()
                    && self.pid.is_none()
                    && self.shell.is_none()
                    && self.window_properties.is_none()
                    && self.layout != NodeLayout::None
                {
                    RefinedNodeType::FloatingContainer
                } else if self.node_type == NodeType::Con && self.pid.is_some() {
                    RefinedNodeType::Window
                } else if self.node_type == NodeType::FloatingCon && self.pid.is_some() {
                    RefinedNodeType::FloatingWindow
                } else {
                    panic!(
                            "Boom, don't know what type of node this is:\nid: {}\nnode_type: {:?}\n{:?}",
                            self.id, self.node_type, self
                        )
                }
            }
        }
    }
}
