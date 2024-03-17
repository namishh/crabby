use anyhow::Context;
use dotenv::dotenv;
mod utils {
    pub mod utils; // Import from the utils.rs file
}
use futures::TryStreamExt;
use poise::async_trait;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::User;

use mongodb::bson::doc;
use mongodb::bson::Document;
use mongodb::Client as MongoClient;
use mongodb::Collection;
use poise::builtins;

use chrono::Utc;
use reqwest::Client;
use serde_json::Value;
use serenity::all::OnlineStatus;
use serenity::builder::CreateEmbed;
use serenity::client;
use serenity::model::colour::Colour;
use serenity::model::gateway::GatewayIntents;
use serenity::model::gateway::Ready;
use std::sync::Arc;

pub struct MongoClientKey;

impl poise::serenity_prelude::prelude::TypeMapKey for MongoClientKey {
    type Value = Arc<MongoClient>;
}

struct Handler {}

impl Handler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl client::EventHandler for Handler {
    async fn ready(&self, _ctx: client::Context, ready: Ready) {
        println!("Working as {}", ready.user.tag());
    }
}

// // MODERATION COMMANDS

// KICK
#[poise::command(
    slash_command,
    prefix_command,
    help_text_fn = "help_kick",
    guild_only,
    required_permissions = "KICK_MEMBERS"
)]
async fn kwick(
    ctx: utils::utils::Context<'_>,
    #[description = "Offendor"]
    #[rename = "criminal"]
    user: User,
    #[description = "Reason?"]
    #[rest]
    reason: String,
) -> Result<(), utils::utils::Error> {
    if ctx.author() != &user {
        create_mod_action_in_database(
            "KICK".to_string(),
            user.clone(),
            reason.clone(),
            ctx.clone(),
        )
        .await;
        let guild = ctx.guild().context("Failed to fetch guild")?.clone();
        guild.kick_with_reason(ctx.http(), &user, &reason).await?;
        ctx.say(format!("**Kicked** user {}. Reason: {}", &user, &reason))
            .await?;
    }
    Ok(())
}

fn help_kick() -> String {
    String::from(
        "\
Example usage:
uwu kick @<mention> <reason>",
    )
}

// BAN
#[poise::command(
    slash_command,
    prefix_command,
    help_text_fn = "help_ban",
    guild_only,
    required_permissions = "BAN_MEMBERS"
)]
async fn ban(
    ctx: utils::utils::Context<'_>,
    #[description = "Offendor"]
    #[rename = "criminal"]
    user: User,
    #[description = "Should I delete their recent messages?"]
    #[rename = "delete"]
    #[flag]
    delete_messages: bool,
    #[description = "Reason?"]
    #[rest]
    reason: String,
) -> Result<(), utils::utils::Error> {
    if ctx.author() != &user {
        create_mod_action_in_database("BAN".to_string(), user.clone(), reason.clone(), ctx.clone())
            .await;
        let guild = ctx.guild().context("Failed to fetch guild")?.clone();
        guild
            .ban_with_reason(
                ctx.http(),
                &user,
                if delete_messages { 1 } else { 0 },
                &reason,
            )
            .await?;
        ctx.say(format!("**Banned** user {}. Reason: {}", &user, &reason))
            .await?;
    }
    Ok(())
}

fn help_ban() -> String {
    String::from(
        "\
Example usage:
uwu ban @<mention> <reason>",
    )
}

// UNBAN
#[poise::command(
    slash_command,
    prefix_command,
    help_text_fn = "help_unban",
    guild_only,
    required_permissions = "BAN_MEMBERS"
)]
async fn unbwan(
    ctx: utils::utils::Context<'_>,
    #[description = "Rehabed person?"]
    #[rename = "good_person"]
    user: User,
) -> Result<(), utils::utils::Error> {
    if ctx.author() != &user {
        create_mod_action_in_database(
            "UNBAN".to_string(),
            user.clone(),
            "Unbanned".to_string(),
            ctx.clone(),
        )
        .await;
        let guild = ctx.guild().context("Failed to fetch guild")?.to_owned();
        guild.unban(ctx.http(), user.id).await?;
        ctx.say(format!("**Unbanned** {}", user.id)).await?;
    }
    Ok(())
}

fn help_unban() -> String {
    String::from(
        "\
Example usage:
uwu unban <userid>",
    )
}

// OFFENCES
#[poise::command(
    slash_command,
    prefix_command,
    help_text_fn = "help_unban",
    guild_only,
    required_permissions = "BAN_MEMBERS"
)]
async fn ofwences(
    ctx: utils::utils::Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), utils::utils::Error> {
    let offenses = match get_user_offenses(user.expect("NO USER").clone(), ctx.clone()).await {
        Ok(offenses) => offenses,
        Err(err) => {
            ctx.say("Error fetching offenses").await?;
            return Err(err);
        }
    };

    let mut embed = CreateEmbed::default().title("USER OFFENCES");
    let mut m = CreateEmbed::default();
    // Loop through offenses and add fields to the embed
    if !offenses.is_empty() {
        for offense in offenses {
            let offense_type = offense.get_str("type").unwrap_or("Unknown Type");
            let reason = offense.get_str("reason").unwrap_or("No reason provided");
            let responsible_mod = offense
                .get_str("responsible_mod")
                .unwrap_or("Unknown Moderator");

            embed = embed.field(
                offense_type,
                format!("**Reason:** {}\n**Moderator:** {}", reason, responsible_mod),
                true,
            );

            m = embed.clone();
        }

        let builder = poise::CreateReply::default().embed(m);
        ctx.send(builder).await?;
    } else {
        ctx.say("User Is Clean").await?;
    }

    Ok(())
}

// // MISC COMMANDS

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

#[poise::command(slash_command, prefix_command, guild_only)]
async fn sweverwinfo(ctx: utils::utils::Context<'_>) -> Result<(), utils::utils::Error> {
    let guild = ctx.guild().context("Failed to fetch guild")?.to_owned();
    let icon_url = guild
        .icon_url()
        .map(|url| url.to_string())
        .unwrap_or_default();

    let mut embed = CreateEmbed::default()
        .title(&guild.name)
        .image(icon_url)
        .color(Colour::BLURPLE);

    embed = embed.field("Members", guild.members.len().to_string(), true);
    embed = embed.field("Channels", guild.channels.len().to_string(), true);
    embed = embed.field("Roles", guild.roles.len().to_string(), true);
    embed = embed.field("ID", guild.id.to_string(), false);
    let builder = poise::CreateReply::default().embed(embed);
    let _msg = ctx.send(builder).await?;
    Ok(())
}

// // FUN COMMANDS
#[poise::command(slash_command, prefix_command)]
async fn wandomanime(ctx: utils::utils::Context<'_>) -> Result<(), utils::utils::Error> {
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
async fn main() {
    dotenv().ok();
    // Initialize the bot with your Discord bot token

    let uri = std::env::var("DATABASE_URL").expect("No database url");
    let mongo = MongoClient::with_uri_str(uri)
        .await
        .expect("Error while connecting");

    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::all();

    let mongo_arc = Arc::new(mongo);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                age(),
                wandomanime(),
                kwick(),
                ban(),
                unbwan(),
                ofwences(),
                sweverwinfo(),
            ],
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
                ctx.data
                    .write()
                    .await
                    .insert::<MongoClientKey>(Arc::clone(&mongo_arc));
                Ok(utils::utils::Data {
                    mongo: Arc::clone(&mongo_arc),
                })
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

async fn create_mod_action_in_database(
    action: String,
    user: User,
    reason: String,
    ctx: utils::utils::Context<'_>,
) {
    let db = ctx.data().mongo.clone();
    let client_ref: &MongoClient = db.as_ref();
    let db_ref = client_ref.database("crabby");
    let collection: Collection<Document> = db_ref.collection("ModNotes");
    let current_time = Utc::now();
    let user_info = format!("{}", user.tag());
    let mod_info = format!("{}", ctx.author().tag());
    let reason_str = reason.to_string();

    let document = doc! {
        "type": action,
        "user": user_info,
        "reason": reason_str,
        "at": current_time.to_rfc3339(),
        "responsible_mod": mod_info,
    };

    let _ = collection.insert_one(document, None).await;
}

async fn get_user_offenses(
    user: User,
    ctx: utils::utils::Context<'_>,
) -> Result<Vec<Document>, utils::utils::Error> {
    let db = ctx.data().mongo.clone();
    let client_ref: &MongoClient = db.as_ref();
    let db_ref = client_ref.database("crabby");
    let collection: Collection<Document> = db_ref.collection("ModNotes");

    let mut cursor = collection.find(doc! { "user": user.tag() }, None).await;
    let mut current_presents: Vec<Document> = Vec::new();

    while let Ok(cursor) = &mut cursor {
        if let Some(doc) = cursor.try_next().await? {
            current_presents.push(doc);
        } else {
            break; // No more documents in the cursor, exit the loop
        }
    }

    Ok(current_presents)
}
