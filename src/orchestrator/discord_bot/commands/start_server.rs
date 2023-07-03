use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::CommandDataOption,
};
use tokio::sync::{mpsc, oneshot};

use crate::orchestrator::discord_bot::discord_command_adapter::{
    HostCommand, ServerCommand, ServerStatus,
};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("start_server")
        .description("Starts the minecraft server")
}

pub async fn run(
    _options: &[CommandDataOption],
    command_sender: mpsc::Sender<HostCommand>,
) -> String {
    let (tx, rx) = oneshot::channel();

    let send_result = command_sender
        .send(HostCommand {
            server_id: 0,
            server_command: ServerCommand::QueryStatus(tx),
        })
        .await;

    let server_status = if let Err(err) = send_result {
        println!("Could not retrieve server status in start_server command, assuming the server was stopped. {:?}", err);
        ServerStatus::Stopped // Assume the server is off so that we can send a start command
    } else {
        match rx.await {
            Ok(status) => status,
            Err(err) => {
                println!("Could not retrieve server status in start_server command, assuming the server was stopped. {:?}", err);
                ServerStatus::Stopped
            }
        }
    };

    match server_status {
        ServerStatus::Starting => "Servern startars redan. Noop",
        ServerStatus::Running => "Server är redan igång. Noop",
        ServerStatus::Stopping => "Servern stannar. Försök igen när servern är avstängd. Noop",
        ServerStatus::Stopped => {
            command_sender
                .send(HostCommand {
                    server_id: 0,
                    server_command: ServerCommand::Start,
                })
                .await
                .unwrap();
            "Startar servern"
        }
    }
    .to_string()
}
