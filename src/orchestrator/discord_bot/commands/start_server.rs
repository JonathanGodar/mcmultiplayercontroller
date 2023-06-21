use std::{borrow::Borrow, env, time::Duration};

use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    model::prelude::interaction::application_command::CommandDataOption,
};
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot, watch,
};

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

    command_sender.send(HostCommand {
        server_id: 0,
        server_command: ServerCommand::QueryStatus(tx),
    });

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

    // let status = if let rx.await {

    // };

    // "Not implemented".to_string()
    // let status = (*mc_host_status.borrow()).clone();
    // match status {
    //     MCHostStatus::Online => {
    //         command_sender.send(controllerp::Command::StartServer).await.unwrap();
    //         "Starting minecraft server"
    //     },
    //     MCHostStatus::Offline => {

    //         tokio::spawn(async move {
    //             let mac_addr = MacAddr::from(env::var("wol_mac").expect("wol_mac env var not set").as_str());
    //             let result = crate::wol::try_until_with_timeout(mac_addr, || {
    //                 *mc_host_status.borrow() != MCHostStatus::Offline
    //             }, Duration::from_secs(60)).await;

    //             if result.is_ok() {
    //                 println!("Queueing command");
    //                 command_sender.send(controllerp::Command::StartServer).await.unwrap();
    //                 println!("Command queued");
    //             }
    //         });

    //         "Starting the minecraft hosting computer"
    //     },
    //     MCHostStatus::Running => {
    //         "The minecraft server is already running"
    //     }
    //     MCHostStatus::ShuttingDown => {
    //         "The server is currently shutting down. Wait for it to be offline and then retry this command"
    //     },
    // }.into()
}
