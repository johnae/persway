use swayipc_async::WindowEvent;

pub trait WindowEventHandler {
    async fn handle(&mut self, event: Box<WindowEvent>);
}
