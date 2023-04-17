use serenity::model::application::interaction::Interaction;
use serenity::{
    async_trait,
    model::prelude::{GuildId, Message, Ready},
    prelude::*,
};

mod commands;
// mod wol;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;

    let token = std::env::var("discord_token").expect("You have to provide a discord bot key");
    let mut client = Client::builder(token, GatewayIntents::non_privileged())
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("An error  occured while running the client: {:?}", why);
    }

    Ok(())
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            println!("Recieved command interaction");
            let content = match command.data.name.as_str() {
                "start_server" => commands::start_server::run(&command.data.options).await,
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
            commands.create_application_command(|command| commands::start_server::register(command))
        })
        .await;

        println!(
            "The following guild commands have been registered: {:#?}",
            commands
        );
    }
}
