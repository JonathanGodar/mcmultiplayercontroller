use std::{env, borrow::Borrow, time::Duration};

use serenity::{builder::CreateApplicationCommand, model::prelude::interaction::application_command::CommandDataOption};
use tokio::sync::{mpsc::Sender, watch};

use crate::{controller::controllerp, MCHostStatus, wol::MacAddr};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("start_server").description("Starts the minecraft server")
}


pub async fn run(_options: &[CommandDataOption], command_sender: Sender<controllerp::Command>, mc_host_status: watch::Receiver<MCHostStatus>) -> String {
    let status = (*mc_host_status.borrow()).clone();
    match status {
        MCHostStatus::Online => {
            command_sender.send(controllerp::Command::StartServer).await.unwrap();
            "Starting minecraft server"
        },
        MCHostStatus::Offline => {

            tokio::spawn(async move {
                let mac_addr = MacAddr::from(env::var("wol_mac").expect("wol_mac env var not set").as_str());
                let result = crate::wol::try_until_with_timeout(mac_addr, || {
                    *mc_host_status.borrow() != MCHostStatus::Offline
                }, Duration::from_secs(60)).await;

                if result.is_ok() {
                    println!("Queueing command");
                    command_sender.send(controllerp::Command::StartServer).await.unwrap();
                    println!("Command queued");
                }
            });


            "Starting the minecraft hosting computer"
        },
        MCHostStatus::Running => {
            "The minecraft server is already running"
        } 
        MCHostStatus::ShuttingDown => {
            "The server is currently shutting down. Wait for it to be offline and then retry this command"
        },
    }.into()
}

