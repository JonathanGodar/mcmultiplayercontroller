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
        .name("server_status")
        .description("Prints the server status")
}

pub async fn run(
    _options: &[CommandDataOption],
    command_sender: mpsc::Sender<HostCommand>,
) -> String {
    let (tx, rx) = oneshot::channel();

    command_sender
        .send(HostCommand {
            server_id: 0,
            server_command: ServerCommand::QueryStatus(tx),
        })
        .await
        .unwrap();

    let status_res = rx.await;
    match status_res {
        Ok(status) => match status {
            ServerStatus::Starting => "Servern startas",
            ServerStatus::Running => "Servern är igång",
            ServerStatus::Stopping => "Servern stängs av",
            ServerStatus::Stopped => "Servern är avstängd",
        },
        Err(err) => {
            println!("Error while querying server status: {:?}", err);
            "Internt fel, kunde inte efterfråga serverstatus"
        }
    }
    .to_string()
}
