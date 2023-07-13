use std::{collections::HashMap, fs::FileType, sync::Arc};

use futures_util::{future::join_all, StreamExt};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
};
use tokio_stream::wrappers::ReadDirStream;
use tokio_util::sync::CancellationToken;

use super::{
    constants::SERVERS_PATH, server::Server, server_installation::ServerInstallationId,
    server_installations::ServerInstallations,
};

#[derive(Debug)]
pub enum Command {
    Start(u64, oneshot::Sender<Result<(), &'static str>>),
    Create(
        u64,
        String,
        ServerInstallationId,
        oneshot::Sender<Result<(), &'static str>>,
    ),
    QueryStatus(u64, oneshot::Sender<Option<ServerStatus>>),
}

#[derive(Debug)]
pub enum ServerStatus {
    Running,
    Stopped,
}

#[derive(Debug)]
enum ServerStatusWithServerHandle {
    Running(JoinHandle<()>),
    Stopped(Server),
}

#[derive(Debug, Clone)]
pub enum ServersManagerEvent {
    AllServersStopped,
    ServerStarted(u64),
}

impl ServerStatusWithServerHandle {
    fn is_running(&self) -> bool {
        match self {
            ServerStatusWithServerHandle::Running(_) => true,
            ServerStatusWithServerHandle::Stopped(_) => false,
        }
    }

    fn without_handle(&self) -> ServerStatus {
        match self {
            ServerStatusWithServerHandle::Running(_) => ServerStatus::Running,
            ServerStatusWithServerHandle::Stopped(_) => ServerStatus::Stopped,
        }
    }
}

pub struct ServersManager {
    servers: HashMap<u64, ServerStatusWithServerHandle>,
    server_installations: Arc<ServerInstallations>,
    event_emitter: broadcast::Sender<ServersManagerEvent>,
}

impl ServersManager {
    pub fn new(
        server_installations: ServerInstallations,
        event_emitter: broadcast::Sender<ServersManagerEvent>,
    ) -> Self {
        Self {
            server_installations: Arc::new(server_installations),
            servers: HashMap::new(),
            event_emitter,
        }
    }

    async fn load_servers() -> HashMap<u64, ServerStatusWithServerHandle> {
        let mut possible_server_dir_entries =
            ReadDirStream::new(tokio::fs::read_dir(SERVERS_PATH.as_path()).await.unwrap());

        // Todo refactor to a function: get_available_server_id:s
        let mut available_server_ids = vec![];
        while let Some(entry_result) = possible_server_dir_entries.next().await {
            match entry_result {
                Ok(entry) => {
                    if let Ok(file_type) = entry.file_type().await {
                        if !file_type.is_dir() {
                            continue;
                        }

                        let maybe_server_id: Option<u64> =
                            entry.file_name().to_str().unwrap().parse().ok();
                        if let Some(server_id) = maybe_server_id {
                            available_server_ids.push(server_id);
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        let servers: HashMap<u64, ServerStatusWithServerHandle> =
            tokio_stream::iter(available_server_ids.into_iter().map(|id| Server::load(id)))
                .then(|server_result| async move { server_result.await })
                .filter_map(|server_result| async move {
                    let server = server_result.ok()?;
                    Some((server.id, ServerStatusWithServerHandle::Stopped(server)))
                })
                .collect()
                .await;

        println!("ServersManager.load_servers: Loaded servers: {:?}", servers);
        servers
    }

    pub async fn run(
        &mut self,
        mut command_rx: mpsc::Receiver<Command>,
        extarnal_shutdown: CancellationToken,
    ) {
        let servers = Self::load_servers().await;
        self.servers = servers;

        let (exited_servers_tx, mut exited_servers_rx) = mpsc::unbounded_channel::<Server>();
        let internal_shutdown = CancellationToken::new();
        loop {
            select! {
                maybe_command = command_rx.recv() => {
                    match maybe_command {
                        Some(command) => {self.handle_command(command, exited_servers_tx.clone(), internal_shutdown.clone()).await;},
                        None => {println!("ServersManager: Exiting since the command pipe closed."); break;}
                    }
                },
                _ = extarnal_shutdown.cancelled() => {
                    println!("ServerManager: Received a shutdown signal");
                    break;
                },
                Some(server) = exited_servers_rx.recv() => {
                    self.servers.insert(server.id, ServerStatusWithServerHandle::Stopped(server));

                    if self.servers.iter().all(|server_status| matches!(server_status.1, ServerStatusWithServerHandle::Stopped(_))) {
                        self.event_emitter.send(ServersManagerEvent::AllServersStopped).unwrap();
                    };

                }
            }
        }

        println!("ServersManager: Exited server_manager main run loop. Sending shutdown signal");
        internal_shutdown.cancel();

        println!("ServersManager: Joining servers");

        let handles = self
            .servers
            .iter_mut()
            .filter_map(|server_status| match server_status.1 {
                ServerStatusWithServerHandle::Running(handle) => Some(handle),
                _ => None,
            });

        let join_results = join_all(handles).await;
        join_results.into_iter().for_each(|result| result.unwrap());
        println!("ServersManager: All servers were sucessfully joined");
        println!("ServersManager: Exited");
    }

    fn is_running(&self, id: u64) -> bool {
        let server = self.servers.get(&id);
        match server {
            Some(server_status) => server_status.is_running(),
            None => false,
        }
    }

    async fn handle_command(
        &mut self,
        command: Command,
        exited_servers_tx: mpsc::UnboundedSender<Server>,
        shutdown_servers: CancellationToken,
    ) {
        match command {
            Command::Start(id, response) => {
                let resp = self
                    .handle_start_command(id, exited_servers_tx, shutdown_servers.clone())
                    .await;
                response.send(resp).unwrap();
            }
            Command::Create(id, name, installation, response_sender) => {
                let resp = self.handle_create_command(id, name, installation).await;
                response_sender.send(resp).unwrap();
            }
            Command::QueryStatus(id, response_sender) => {
                let resp = self.servers.get(&id).map(|s| s.without_handle());

                response_sender.send(resp).unwrap();
            }
        }
    }

    async fn handle_create_command(
        &mut self,
        id: u64,
        name: String,
        installation: ServerInstallationId,
    ) -> Result<(), &'static str> {
        if self.servers.contains_key(&id) {
            return Err("Could not create the server since it is already loaded");
        }

        let create_server_result = Server::create(id, name, installation).await;
        let resp = match create_server_result {
            Ok(server) => {
                self.servers
                    .insert(id, ServerStatusWithServerHandle::Stopped(server));
                Ok(())
            }
            Err(err) => Err(err),
        };

        resp
    }

    async fn handle_start_command(
        &mut self,
        id: u64,
        exited_servers_tx: mpsc::UnboundedSender<Server>,
        shutdown_servers: CancellationToken,
    ) -> Result<(), &'static str> {
        let server_installations = self.server_installations.clone();

        let server_status = self
            .servers
            .remove(&id)
            .ok_or("Could not start server because it was not loaded")?;

        // "Wierdly" written that it is impossible to forget to add the servers back incase of an addition of more variants to the server state enum
        let (status, resp) = match server_status {
            ServerStatusWithServerHandle::Running(join_handle) => (
                ServerStatusWithServerHandle::Running(join_handle),
                Err("Could not start server since it is already running"),
            ),
            ServerStatusWithServerHandle::Stopped(mut server) => {
                let join_handle = tokio::spawn(async move {
                    server
                        .run(server_installations, shutdown_servers)
                        .await
                        // TODO Handle startup errors instead of unwrapping
                        .unwrap();

                    _ = exited_servers_tx.send(server);
                });

                (ServerStatusWithServerHandle::Running(join_handle), Ok(()))
            }
        };

        self.servers.insert(id, status);
        resp
    }
}
