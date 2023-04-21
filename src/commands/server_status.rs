use serenity::{builder::CreateApplicationCommand, model::prelude::interaction::application_command::CommandDataOption};
use tokio::sync::{watch};

use crate::{MCHostStatus};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("server_status").description("Prints the server status")
}


pub async fn run(_options: &[CommandDataOption], mc_host_status: watch::Receiver<MCHostStatus>) -> String {
    let status = (*mc_host_status.borrow()).clone();

    format!("The minecraft host is currently, {:?}", status)
}

