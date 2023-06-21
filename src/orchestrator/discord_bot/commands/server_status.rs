use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::CommandDataOption,
};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("server_status")
        .description("Prints the server status")
}

pub async fn run(_options: &[CommandDataOption]) -> String {
    todo!("Hello:?")
    // format!("The minecraft host is currently, {:?}", status)
}
