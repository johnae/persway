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

    async fn on_window_focus(&mut self, event: WindowEvent) -> Result<()> {
        log::debug!("spiral manager handling event: {:?}", event.change);
        let focused_node = &event.container;
        let ws = focused_node.get_workspace().await?;
        if ws.name == utils::PERSWAY_TMP_WORKSPACE {
            log::debug!("skip spiral layout of tmp workspace");
            return Ok(());
        }
        if !(focused_node.is_floating_window()
            || focused_node.is_floating_container()
            || focused_node.is_full_screen()
            || focused_node.is_stacked().await?
            || focused_node.is_tabbed().await?)
        {
            let cmd = if focused_node.rect.height > focused_node.rect.width {
                format!(
                    "[con_id={focused_node_id}] focus; split v",
                    focused_node_id = focused_node.id
                )
            } else {
                format!(
                    "[con_id={focused_node_id}] focus; split h",
                    focused_node_id = focused_node.id
                )
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
                if let Err(e) = self.on_window_focus(*event).await {
                    log::error!("spiral manager, layout err: {}", e);
                };
            }
            _ => log::debug!("spiral manager, not handling event: {:?}", event.change),
        }
    }
}
