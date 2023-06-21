use std::{env, future::Future, pin::Pin, time::Duration};

use futures_core::Stream;
use tokio::{
    select,
    sync::{mpsc, watch},
};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tokio_util::sync::CancellationToken;
use tonic::{async_trait, transport::Server, Request, Response, Status};

use crate::{
    network_utils::wake_on_lan::{send_wol_with_timeout, MacAddr},
    orchestrator::{
        constants::{
            BROADCAST_ADDRESS, GRPC_LISTEN_ADDR_ENV_NAME, HOST_MAC_ADDRESS_ENV_NAME,
            WOL_SEND_FROM_ADDRESS,
        },
        discord_bot::discord_command_adapter::{
            HostCommand, HostManager, ServerCommand, ServerStatus,
        },
    },
};

use super::controller::{controllerp::basics_server::BasicsServer, HostStatus, MyBasics};

pub struct GrpcHostManager {
    // command_reciever: mpsc::Receiver<HostCommand>,
}

impl GrpcHostManager {
    pub fn new(// command_reciever: tokio::sync::mpsc::Receiver<
        //     crate::orchestrator::discord_bot::discord_command_adapter::HostCommand,
        // >,
    ) -> Self {
        Self {}
    }
}

#[async_trait]
impl HostManager for GrpcHostManager {
    fn get_event_emmiter(
        &self,
    ) -> tokio::sync::mpsc::Receiver<
        crate::orchestrator::discord_bot::discord_command_adapter::HostEvent,
    > {
        todo!()
    }

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
        match host_command.server_command {
            ServerCommand::Start => {
                if !host_status.borrow_and_update().host_is_connected() {
                    // TODO handle errors
                    start_host(host_status.clone()).await.unwrap();
                }

                grpc_command_tx.send(host_command).await.unwrap();
            }
            ServerCommand::ApplyChanges(_) => todo!(),
            ServerCommand::Stop => {}
            ServerCommand::QueryStatus(_) => todo!(),
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
