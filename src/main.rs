use anyhow::Context;
use dotenv::dotenv;
mod utils {
    pub mod utils; // Import from the utils.rs file
}
use poise::async_trait;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::User;
use std::any::type_name;

use reqwest::Client;
use serde_json::Value;
use serenity::all::OnlineStatus;
use serenity::builder::CreateEmbed;
use serenity::model::colour::Colour;
use serenity::model::gateway::GatewayIntents;
use serenity::model::gateway::Ready;
use std::sync::Arc;
use poise::builtins;
use serenity::client;
use poise::serenity_prelude::prelude::TypeMapKey;
use mongodb::Client as MongoClient;

pub struct MongoClientKey;

impl TypeMapKey for MongoClientKey {
    type Value = MongoClient;
}

struct Handler {}

impl Handler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl client::EventHandler for Handler {
    async fn ready(&self, _ctx: client::Context, _ready: Ready) {
        println!("Working");
    }
}

// MODERATION COMMANDS
#[poise::command(
    slash_command,
    prefix_command,
    help_text_fn = "help_kick",
    guild_only,
    required_permissions = "KICK_MEMBERS"
)]
async fn kick(
    ctx: utils::utils::Context<'_>,
    #[description = "Offendor"]
    #[rename = "criminal"]
    user: User,
    #[description = "Reason?"]
    #[rest]
    reason: String,
) -> Result<(), utils::utils::Error> {
    println!("Type of ctx.data(): {}", type_name::<_>(ctx.data()));


    let guild = ctx.guild().context("Failed to fetch guild")?.clone();
    guild.kick_with_reason(ctx.http(), &user, &reason).await?;
    ctx.say(format!("**Kicked** user {}. Reason: {}", &user, &reason))
        .await?;
    Ok(())
}

fn help_kick() -> String {
    String::from(
        "\
Example usage:
uwu kick @<mention> <reason>",
    )
}

#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: utils::utils::Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), utils::utils::Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn randomanime(ctx: utils::utils::Context<'_>) -> Result<(), utils::utils::Error> {
    let client = Client::new();
    let response = client
        .get("https://api.jikan.moe/v4/random/anime")
        .send()
        .await;

    match response {
        Ok(res) => {
            let body = res.bytes().await;

            let mut b = String::new();

            let body_str = String::from_utf8(body.expect("Reason").to_vec());

            match body_str {
                Ok(str) => {
                    b = str;
                }
                Err(_err) => {}
            }
            let anime_data: Value = serde_json::from_str(&b).unwrap();

            let title = anime_data["data"]["title"]
                .as_str()
                .unwrap_or("Unknown Title");
            let synopsis = anime_data["data"]["synopsis"]
                .as_str()
                .unwrap_or("No synopsis available");
            let image_url = anime_data["data"]["images"]["jpg"]["large_image_url"]
                .as_str()
                .unwrap_or("");

            let embed = CreateEmbed::new()
                .title(title)
                .description(synopsis)
                .image(image_url)
                .color(Colour::BLURPLE);

            let builder = poise::CreateReply::default().embed(embed);
            let _msg = ctx.send(builder).await?;
        }
        Err(_) => {
            let _ = ctx.say("Failed to fetch anime data.");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main()  {
    dotenv().ok();
    // Initialize the bot with your Discord bot token

    let uri = std::env::var("DATABASE_URL").expect("No database url");
    let mongo = MongoClient::with_uri_str(uri).await.expect("Error while connecting");


    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::all();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![age(), randomanime(), kick()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("uwu".into()),
                case_insensitive_commands: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                builtins::register_globally(ctx, &framework.options().commands).await?;
                ctx.data.write().await.insert::<MongoClientKey>(mongo);
                Ok(utils::utils::Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .activity(serenity::gateway::ActivityData::listening("uwu"))
        .status(OnlineStatus::Online)
        .framework(framework)
        .event_handler(Handler::new())
        .await;

    client.unwrap().start().await.unwrap();
}
