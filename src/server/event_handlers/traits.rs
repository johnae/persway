use async_trait::async_trait;
use swayipc_async::WindowEvent;

#[async_trait]
pub trait WindowEventHandler {
    async fn handle(&mut self, event: Box<WindowEvent>);
}
