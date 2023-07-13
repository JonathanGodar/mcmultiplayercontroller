use std::io::Write;

use futures_util::join;
use mcmultiplayercontroller::mchost::{
    grpc_orchestrator_connection::{self, GrpcOrchestratorConnection},
    server_installations::ServerInstallations,
    servers_manager::ServersManager,
    system_shutdown::system_shutdown_handler,
};
use tokio::{
    process, signal,
    sync::{broadcast, mpsc},
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    if let Err(err) = dotenvy::dotenv() {
        println!("WARNING: Error while loading .env: {}", err);
    }

    let server_installations = ServerInstallations::load().await;

    let (server_manager_event_tx, server_manager_event_rx) = broadcast::channel(128);
    let mut servers_manager = ServersManager::new(server_installations, server_manager_event_tx);

    let shutdown = CancellationToken::new();
    let (command_tx, command_rx) = mpsc::channel(2);
    let mut grpc_orchestrator_connection = GrpcOrchestratorConnection::new(command_tx);

    let system_shutdown_signal = CancellationToken::new();

    tokio::spawn(shutdown_signal_handler(shutdown.clone()));
    let servers_manager_fut = servers_manager.run(command_rx, shutdown.clone());
    let grpc_orchestrator_connection = grpc_orchestrator_connection.run(shutdown.clone());
    let system_shutdown_handler = system_shutdown_handler(
        server_manager_event_rx,
        system_shutdown_signal.clone(),
        shutdown,
    );

    join!(
        servers_manager_fut,
        grpc_orchestrator_connection,
        system_shutdown_handler
    );

    if system_shutdown_signal.is_cancelled() {
        let output = process::Command::new("systemctl")
            .arg("poweroff")
            .output()
            .await;
        println!("Tried to shut down, {:?}", output);
    }
}

async fn shutdown_signal_handler(shutdown_signal: CancellationToken) {
    signal::ctrl_c().await.unwrap();
    shutdown_signal.cancel();
    println!("ShutdownSignalHandler: Recieved exit signal. Shutting down the application");
}
