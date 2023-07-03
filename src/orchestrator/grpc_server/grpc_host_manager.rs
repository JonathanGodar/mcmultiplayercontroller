use std::{env, time::Duration};

use tokio::{
    select,
    sync::{mpsc, watch},
};

use tonic::{async_trait, transport::Server};

use crate::{
    network_utils::wake_on_lan::{send_wol_with_timeout, MacAddr},
    orchestrator::{
        constants::{
            BROADCAST_ADDRESS, GRPC_LISTEN_ADDR_ENV_NAME, HOST_MAC_ADDRESS_ENV_NAME,
            WOL_SEND_FROM_ADDRESS,
        },
        discord_bot::discord_command_adapter::{HostCommand, HostManager, ServerCommand},
    },
};

use super::controller::{controllerp::basics_server::BasicsServer, HostStatus, MyBasics};

pub struct GrpcHostManager {}

impl GrpcHostManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl HostManager for GrpcHostManager {
    async fn start(
        &mut self,
        discord_command_rx: mpsc::Receiver<HostCommand>,
    ) -> Result<(), &'static str> {
        let addr = env::var(GRPC_LISTEN_ADDR_ENV_NAME)
            .expect("Env var listen_address must be set")
            .parse()
            .unwrap(); // TODO remove unwrap

        let (grpc_command_tx, host_manager_rx) = mpsc::channel::<HostCommand>(8);
        let service = MyBasics::new(host_manager_rx);

        println!(
            "Starting grpc server on: {}",
            env::var(GRPC_LISTEN_ADDR_ENV_NAME).unwrap()
        );

        let host_status = service.get_status_watcher();
        select! {
            _ = Server::builder()
                .tcp_keepalive(Some(Duration::from_secs(59)))
                .add_service(BasicsServer::new(service))
                .serve(addr) => {
                    println!("Discord server exited");
                },
            _ = handle_commands(discord_command_rx, grpc_command_tx, host_status) => {
                println!("Command handler in grpc_host_manager exited");
            }
        }

        Ok(())
    }
}

async fn handle_commands(
    mut discord_command_rx: mpsc::Receiver<HostCommand>,
    grpc_command_tx: mpsc::Sender<HostCommand>,
    mut host_status: watch::Receiver<HostStatus>,
) {
    while let Some(host_command) = discord_command_rx.recv().await {
        println!("Handling command: {:?}", host_command);
        match host_command.server_command {
            ServerCommand::Start => {
                if !host_status.borrow_and_update().host_is_connected() {
                    // TODO handle errors
                    let result = start_host(host_status.clone()).await;
                    if let Err(err) = result {
                        println!(
                            "Error starting server {:?}: {:?}",
                            host_command.server_id, err
                        );
                    }
                }
                grpc_command_tx.send(host_command).await.unwrap();
            }
            ServerCommand::ApplyChanges(_) => {
                todo!("ApplyChanges is not yet implemented in gprc_host_manager.rs")
            }
            ServerCommand::Stop => {
                todo!("Stop is not yet implemented in gprc_host_manager.rs")
            }
            ServerCommand::QueryStatus(server_status_sender) => {
                let server_status = host_status
                    .borrow()
                    .get_server_status(host_command.server_id);

                server_status_sender.send(server_status).unwrap();
            }
        }
    }

    println!("grpc_host_managr command handler exited");
}

async fn start_host(mut host_status: watch::Receiver<HostStatus>) -> Result<(), &'static str> {
    let host_activated = async move {
        if host_status.borrow_and_update().host_is_connected() {
            return true;
        }

        loop {
            let result = host_status.changed().await;

            if result.is_err() {
                return false;
            }

            if host_status.borrow().host_is_connected() {
                return true;
            };
        }
    };

    send_wol_with_timeout(
        MacAddr::from(env::var(HOST_MAC_ADDRESS_ENV_NAME).unwrap().as_str()),
        env::var(WOL_SEND_FROM_ADDRESS).unwrap(),
        env::var(BROADCAST_ADDRESS).unwrap(),
        host_activated,
        Duration::from_secs(60),
    )
    .await
}
