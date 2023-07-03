use futures::Stream;
use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};

use controllerp::basics_server::Basics;
use tokio::{
    select,
    sync::{mpsc, watch, Mutex},
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::orchestrator::{
    discord_bot::discord_command_adapter::{HostCommand, ServerCommand, ServerStatus},
    grpc_server::controller::controllerp::Command,
};

use self::controllerp::{ControllerCommands, HelloReply, HelloRequest};

pub mod controllerp {
    tonic::include_proto!("controllerp");
}

#[derive(Debug)]
pub struct MyBasics {
    command_rx: Arc<Mutex<mpsc::Receiver<HostCommand>>>,
    host_status_updater: Arc<std::sync::Mutex<watch::Sender<HostStatus>>>,
    host_status: watch::Receiver<HostStatus>,
}

#[derive(Debug)]
pub enum HostStatus {
    Offline,
    Online(HashMap<u64, ServerStatus>), // RunningServers(Vec<u64>),
}

impl HostStatus {
    pub fn host_is_connected(&self) -> bool {
        !matches!(self, &HostStatus::Offline)
    }

    pub fn get_server_status(&self, server_id: u64) -> ServerStatus {
        match self {
            HostStatus::Offline => ServerStatus::Stopped,
            HostStatus::Online(servers) => servers
                .get(&server_id)
                .unwrap_or(&ServerStatus::Stopped)
                .clone(),
        }
    }
}

impl MyBasics {
    pub(crate) fn new(command_rx: mpsc::Receiver<HostCommand>) -> Self {
        let (tx, rx) = watch::channel(HostStatus::Offline);

        Self {
            command_rx: Arc::new(Mutex::new(command_rx)),
            host_status: rx,
            host_status_updater: Arc::new(std::sync::Mutex::new(tx)),
        }
    }

    async fn handle_host_connection(
        tx_to_host: mpsc::Sender<Result<Command, Status>>,
        command_rx: Arc<Mutex<mpsc::Receiver<HostCommand>>>,
    ) {
        let tx_to_host = Arc::new(Mutex::new(tx_to_host));

        let heart_beat_task = {
            let tx_to_host = tx_to_host.clone();
            tokio::spawn(async move {
                loop {
                    {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    let tx = tx_to_host.lock().await;
                    let result = tx.send(Ok(Command::HeartBeat)).await;
                    if result.is_err() {
                        break;
                    }
                }
                println!("Heat breat stopped, quitting!");
            })
        };

        let command_handler = async {
            let mut command_rx_lock = command_rx.lock().await;
            while let Some(command) = command_rx_lock.recv().await {
                let tx = tx_to_host.lock().await;

                // TODO grpc_host_manager currently handles ServerCommand::QueryStatus. Make grpc_host_manager handle all the ServerCommands and let this part of the code take some other enum.
                match command.server_command {
                    ServerCommand::Start => {
                        tx.send(Ok(Command::StartServer)).await.unwrap();
                    }
                    _ => todo!(),
                }
            }
        };

        select! {
            _ = heart_beat_task => {},
            _ = command_handler => {}
        }

        println!("Exiting handle_host_connection");
    }

    pub fn get_status_watcher(&self) -> watch::Receiver<HostStatus> {
        self.host_status.clone()
    }
}

#[tonic::async_trait]
impl Basics for MyBasics {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        Ok(Response::new(HelloReply {
            message: format!("Hello world :D, and hello {}", request.get_ref().name),
        }))
    }

    type OnHostStartupStream =
        Pin<Box<dyn Stream<Item = Result<ControllerCommands, Status>> + Send>>;

    async fn on_host_startup(
        &self,
        _: Request<()>,
    ) -> Result<Response<Self::OnHostStartupStream>, Status> {
        if self.host_status.borrow().host_is_connected() {
            return Err(Status::new(
                tonic::Code::Unavailable,
                "Host already connected",
            ));
        } else {
            let lock = self.host_status_updater.lock().unwrap();
            lock.send(HostStatus::Online(HashMap::new())).unwrap();
        }

        let (tx, mut rx) = mpsc::channel::<Result<Command, Status>>(128);

        let command_rx = self.command_rx.clone();
        tokio::spawn(async { Self::handle_host_connection(tx, command_rx).await });

        let (tx, output_rx) = mpsc::channel(8);

        let updater = self.host_status_updater.clone();
        let controller_command_id_handler = async move {
            let mut id_counter = 0;
            while let Some(command_res) = rx.recv().await {
                let tx_err = tx
                    .send(command_res.map(|command| ControllerCommands {
                        id: id_counter,
                        command: command.into(),
                    }))
                    .await;
                id_counter += 1;

                if tx_err.is_err() {
                    break;
                }
            }

            updater.lock().unwrap().send(HostStatus::Offline).unwrap();
        };

        tokio::spawn(controller_command_id_handler);

        let output_stream = ReceiverStream::new(output_rx);
        Ok(Response::new(Box::pin(output_stream)))
    }
}
