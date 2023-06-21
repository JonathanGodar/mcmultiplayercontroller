use std::pin::Pin;

use serenity::{
    async_trait,
    model::{
        application::interaction::Interaction,
        prelude::{GuildId, Ready},
    },
    prelude::*,
    Client,
};
use tokio::{
    select,
    sync::{broadcast, mpsc},
};

use crate::orchestrator::{constants::GUILD_ID_ENV_NAME, discord_bot::commands::start_server};

use self::discord_command_adapter::{HostCommand, HostManager};

use super::constants::DISCORD_TOKEN_ENV_NAME;

mod commands;
pub mod discord_command_adapter;

pub async fn start_discord_bot<T: HostManager + 'static>(mut host_manager: T) {
    let token =
        std::env::var(DISCORD_TOKEN_ENV_NAME).expect("You have to provide a discord bot key");

    let (tx, mut rx) = mpsc::channel(255);

    let mut discord_client = Client::builder(token, GatewayIntents::non_privileged())
        .event_handler(Handler { command_sender: tx })
        .await
        .expect("Error creating discord client");

    println!("Starting dicord client");

    select! {
        val = discord_client.start() => {
            if let Err(err) = val {
                println!("Discord bot exited with err: {}", err);
            } else {
                println!("Discord bot exited");
            }
        }
        val = host_manager.start(rx) =>  {
            if let Err(err) = val {
                println!("Host manager exited with err: {}", err);
            } else {
                println!("Host manager exited");
            }
        }
    }

    println!("Discord_client start exited");
}

struct Handler {
    command_sender: mpsc::Sender<HostCommand>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            println!("recieved command interaction");
            let content = match command.data.name.as_str() {
                "start_server" => {
                    start_server::run(&command.data.options, self.command_sender.clone()).await
                }
                "server_status" => commands::server_status::run(&command.data.options).await,
                _ => String::from("not implemented :/"),
            };

            if let Err(why) = command.create_interaction_response(&ctx.http, |response|  {
        response
            .kind(serenity::model::prelude::interaction::InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|message| message.content(content))
        }).await {
            println!("could not respond to slashcommand :/ {}", why);
        }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{}, is connected!", ready.user.name);

        let guild_id = GuildId(
            std::env::var(GUILD_ID_ENV_NAME)
                .expect("no guild id provided in env")
                .parse()
                .expect("guild_id must be an integer"),
        );
        let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| commands::start_server::register(command))
                .create_application_command(|command| commands::server_status::register(command))
        })
        .await;

        println!(
            "the following guild commands have been registered: {:#?}",
            commands
        );
    }
}
