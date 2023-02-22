use super::super::traits::WindowEventHandler;

use anyhow::Result;
use async_trait::async_trait;
use swayipc_async::{Connection, WindowChange, WindowEvent};

pub struct WindowFocus {
    connection: Connection,
    window_focus_cmd: Option<String>,
    window_focus_leave_cmd: Option<String>,
    previously_focused_id: Option<i64>,
}

impl WindowFocus {
    pub async fn handle(
        event: Box<WindowEvent>,
        window_focus_cmd: Option<String>,
        window_focus_leave_cmd: Option<String>,
    ) {
        if let Ok(mut manager) = Self::new(window_focus_cmd, window_focus_leave_cmd).await {
            manager.handle(event).await;
        }
    }

    pub async fn new(
        window_focus_cmd: Option<String>,
        window_focus_leave_cmd: Option<String>,
    ) -> Result<Self> {
        let connection = Connection::new().await?;
        Ok(Self {
            connection,
            window_focus_cmd,
            window_focus_leave_cmd,
            previously_focused_id: None,
        })
    }

    async fn on_window_focus(&mut self) -> Result<()> {
        if let Some(window_focus_cmd) = &self.window_focus_cmd {
            self.connection.run_command(window_focus_cmd).await?;
        }
        Ok(())
    }

    async fn on_window_focus_leave(&mut self) -> Result<()> {
        if let Some(window_focus_leave_cmd) = &self.window_focus_leave_cmd {
            if let Some(id) = self.previously_focused_id {
                self.connection
                    .run_command(format!("[con_id={id}] {}", window_focus_leave_cmd))
                    .await?;
            }
        }
        Ok(())
    }
}
#[async_trait]
impl WindowEventHandler for WindowFocus {
    async fn handle(&mut self, event: Box<WindowEvent>) {
        match event.change {
            WindowChange::Focus => {
                if let Err(e) = self.on_window_focus_leave().await {
                    log::error!(
                        "workspace window focus manager on_window_focus_leave, err: {}",
                        e
                    );
                };
                if let Err(e) = self.on_window_focus().await {
                    log::error!("workspace window focus manager on_window_focus, err: {}", e);
                };
                self.previously_focused_id = Some(event.container.id);
            }
            WindowChange::Close => {
                if let Err(e) = self.on_window_focus_leave().await {
                    log::error!(
                        "workspace window focus manager on_window_focus_leave, err: {}",
                        e
                    );
                };
                self.previously_focused_id = Some(event.container.id);
            }
            _ => log::debug!(
                "workspace name manager, not handling event: {:?}",
                event.change
            ),
        }
    }
}
