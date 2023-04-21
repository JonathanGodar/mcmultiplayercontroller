use serenity::model::application::interaction::Interaction;
use serenity::{
    async_trait,
    model::prelude::{GuildId,  Ready},
    prelude::*,
};
use tokio::select;
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::sync::watch;

use crate::controller::{start_tonic, controllerp};

mod commands;
mod controller;
mod wol;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;

    let token = std::env::var("discord_token").expect("You have to provide a discord bot key");
    let (command_tx, command_rx) = mpsc::channel::<controllerp::Command>(1);
    let (status_tx, status_rx) = watch::channel(MCHostStatus::Offline);

    let mut client = Client::builder(token, GatewayIntents::non_privileged())
        .event_handler(Handler {
            command_sender: command_tx,
            mc_host_status: status_rx,
        })
        .await
        .expect("Error creating client");

    select! {
        _ = client.start() => {
            println!("Client exited");
        }
        _ = start_tonic(command_rx, status_tx) => {
            println!("Tonic exited");
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MCHostStatus {
    Online,
    Offline,
    Running,
    ShuttingDown,
}

struct Handler {
    command_sender: Sender<controllerp::Command>,
    mc_host_status: watch::Receiver<MCHostStatus>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            println!("Recieved command interaction");
            let content = match command.data.name.as_str() {
                "start_server" => commands::start_server::run(&command.data.options, self.command_sender.clone(), self.mc_host_status.clone()).await,
                "server_status" => commands::server_status::run(&command.data.options, self.mc_host_status.clone()).await,
                _ => String::from("Not implemented :/"),
            };

            if let Err(why) = command.create_interaction_response(&ctx.http, |response|  {
            response
                .kind(serenity::model::prelude::interaction::InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
            }).await {
                println!("Could not respond to slashcommand :/ {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{}, is connected!", ready.user.name);

        let guild_id = GuildId(
            std::env::var("guild_id")
                .expect("No guild id provided in env")
                .parse()
                .expect("guild_id must be an integer"),
        );
        let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands.create_application_command(|command| commands::start_server::register(command)).create_application_command(
                |command| commands::server_status::register(command)
            )
        })
        .await;

        println!(
            "The following guild commands have been registered: {:#?}",
            commands
        );
    }
}
