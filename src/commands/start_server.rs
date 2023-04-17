use serenity::{builder::CreateApplicationCommand, model::prelude::interaction::application_command::CommandDataOption};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("start_server").description("Starts the minecraft server")
}


pub async fn run(_options: &[CommandDataOption]) -> String {
    String::from("Hello world :D")
}

