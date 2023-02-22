use anyhow::{anyhow, Result};
use async_trait::async_trait;
use swayipc_async::{Connection, Node, NodeLayout, NodeType, Workspace};

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
    async fn get_workspace(&self) -> Result<Workspace>;
    fn get_refined_node_type(&self) -> RefinedNodeType;
    async fn get_parent(&self) -> Result<Node>;
    fn iter(&self) -> LinearNodeIterator;
    fn is_root(&self) -> bool;
    fn is_output(&self) -> bool;
    fn is_workspace(&self) -> bool;
    fn is_container(&self) -> bool;
    fn is_floating_container(&self) -> bool;
    fn is_window(&self) -> bool;
    fn is_floating_window(&self) -> bool;
    fn is_full_screen(&self) -> bool;
    async fn is_stacked(&self) -> Result<bool>;
    async fn is_tabbed(&self) -> Result<bool>;
}

#[async_trait]
impl NodeExt for Node {
    fn iter(&self) -> LinearNodeIterator {
        LinearNodeIterator::new(self)
    }

    async fn get_workspace(&self) -> Result<Workspace> {
        let mut connection = Connection::new().await?;
        let tree = connection.get_tree().await?;
        let workspaces = connection.get_workspaces().await?;
        let wsnode = tree
            .find(|n| n.is_workspace() && n.iter().any(|n| n.id == self.id))
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

    async fn get_parent(&self) -> Result<Node> {
        let mut connection = Connection::new().await?;
        let tree = connection.get_tree().await?;
        tree.find(|n| n.nodes.iter().any(|n| n.id == self.id))
            .ok_or_else(|| anyhow!(format!("couldn't find parent of node id: {}", self.id)))
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

    fn is_full_screen(&self) -> bool {
        self.percent.unwrap_or(1.0) > 1.0
    }

    async fn is_stacked(&self) -> Result<bool> {
        let parent = self.get_parent().await?;
        Ok(parent.layout == NodeLayout::Stacked)
    }

    async fn is_tabbed(&self) -> Result<bool> {
        let parent = self.get_parent().await?;
        Ok(parent.layout == NodeLayout::Tabbed)
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
