use std::{env, time::Duration};

use futures_util::StreamExt;
use serenity::model::prelude::StageInstanceCreateEvent;
use tokio::{
    select,
    sync::{mpsc, oneshot},
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use tonic::Status;

use self::controllerp::{basics_client::BasicsClient, ControllerCommands};

use super::{constants::ORCHESTRATOR_ENDPOINT_ENV_NAME, servers_manager::Command};

pub mod controllerp {
    tonic::include_proto!("controllerp");
}

pub struct GrpcOrchestratorConnection {
    servers_manager_command_tx: mpsc::Sender<Command>,
}

impl GrpcOrchestratorConnection {
    pub fn new(servers_manager_command_tx: mpsc::Sender<Command>) -> Self {
        Self {
            servers_manager_command_tx,
        }
    }

    pub async fn run(&mut self, shutdown: CancellationToken) {
        // TODO reconnection attempts
        let mut client = {
            let mut attempts_left = 20;
            let client = loop {
                if attempts_left <= 0 {
                    panic!("Could not connect to gprc basics_client");
                }

                let result =
                    BasicsClient::connect(env::var(ORCHESTRATOR_ENDPOINT_ENV_NAME).unwrap()).await;

                match result {
                    Ok(client) => break client,
                    Err(err) => {
                        println!(
                            "gprc_orchestrator_connection: Could not connect to orchestrator. {}",
                            err
                        )
                    }
                }

                sleep(Duration::from_secs(2)).await;
                attempts_left -= 1;
            };
            client
        };

        // TODO reconnection attempts
        let host_startup_response = client.on_host_startup(()).await.unwrap();

        let mut command_stream = host_startup_response.into_inner();

        loop {
            select! {
                maybe_command_result = command_stream.next() => {
                    match maybe_command_result {
                        Some(command) => {
                            self.handle_command(command).await;
                        },
                        None => {
                            println!("GrpcOrchestratorConnection: Exited because of closed gprc connection");
                            break;
                        },
                    }
                },
                _ = shutdown.cancelled() => {
                    break;
                }
            }
        }
    }

    async fn handle_command(&mut self, controller_commands: Result<ControllerCommands, Status>) {
        let controller_commands = controller_commands.unwrap();
        let command_i = controller_commands.command();
        match command_i {
            controllerp::Command::StartServer => {
                let (resp_tx, response_rx) = oneshot::channel();

                self.servers_manager_command_tx
                    .send(Command::Start(1, resp_tx))
                    .await
                    .unwrap();

                response_rx.await.unwrap().unwrap();
            }
            controllerp::Command::StopServer => todo!(),
            controllerp::Command::HeartBeat => {}
            controllerp::Command::RefreshServers => todo!(),
            controllerp::Command::ActivateHostAutoPowerOff => todo!(),
        }
    }
}
