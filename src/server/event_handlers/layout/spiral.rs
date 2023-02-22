use super::super::traits::WindowEventHandler;
use crate::{node_ext::NodeExt, utils};

use anyhow::Result;
use async_trait::async_trait;
use swayipc_async::{Connection, WindowChange, WindowEvent};

pub struct Spiral {
    connection: Connection,
}

impl Spiral {
    pub async fn handle(event: Box<WindowEvent>) {
        if let Ok(mut manager) = Self::new().await {
            manager.handle(event).await;
        }
    }

    pub async fn new() -> Result<Self> {
        let connection = Connection::new().await?;
        Ok(Self { connection })
    }

    async fn layout(&mut self, event: WindowEvent) -> Result<()> {
        log::debug!("spiral manager handling event: {:?}", event.change);
        let tree = self.connection.get_tree().await?;
        let node = tree
            .find_as_ref(|n| n.id == event.container.id)
            .expect(&format!("no node found with id {}", event.container.id));
        let ws = node.get_workspace().await?;
        if ws.name == utils::PERSWAY_TMP_WORKSPACE {
            log::debug!("skip spiral layout of tmp workspace");
            return Ok(());
        }
        if !(node.is_floating_window()
            || node.is_floating_container()
            || node.is_full_screen()
            || node.is_stacked().await?
            || node.is_tabbed().await?)
        {
            let cmd = if node.rect.height > node.rect.width {
                format!("[con_id={}] focus; split v", node.id)
            } else {
                format!("[con_id={}] focus; split h", node.id)
            };
            log::debug!("spiral layout: {}", cmd);
            self.connection.run_command(cmd).await?;
        };

        Ok(())
    }
}
#[async_trait]
impl WindowEventHandler for Spiral {
    async fn handle(&mut self, event: Box<WindowEvent>) {
        match event.change {
            WindowChange::Focus => {
                if let Err(e) = self.layout(*event).await {
                    log::error!("spiral manager, layout err: {}", e);
                };
            }
            _ => log::debug!("spiral manager, not handling event: {:?}", event.change),
        }
    }
}
