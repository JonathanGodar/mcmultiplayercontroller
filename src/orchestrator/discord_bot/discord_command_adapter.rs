use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct HostCommand {
    pub server_id: u64,
    pub server_command: ServerCommand,
}

#[derive(Debug)]
pub enum ServerCommand {
    Start,
    ApplyChanges(Vec<ServerChange>),
    Stop,
    QueryStatus(oneshot::Sender<ServerStatus>),
}

#[derive(Debug)]
pub enum ServerChange {
    World(String),
}

#[derive(Debug)]
pub struct HostEvent {
    pub server_id: u64,
    pub event: ServerEvent,
}

#[derive(Debug)]
pub enum ServerEvent {
    StatusChange(ServerStatus),
}

#[derive(Debug)]
pub enum ServerStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
}

#[async_trait]
pub trait HostManager: Send + Sync {
    async fn start(
        &mut self,
        command_receiver: mpsc::Receiver<HostCommand>,
    ) -> Result<(), &'static str>;

    fn get_event_emmiter(&self) -> tokio::sync::mpsc::Receiver<HostEvent>;
}
