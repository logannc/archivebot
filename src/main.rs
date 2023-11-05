use std::env;

use itertools::Itertools;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::http::CacheHttp;
use serenity::model::channel::{ChannelType, GuildChannel, Message};
use serenity::model::prelude::GuildContainer;
use serenity::model::user::User;
use serenity::prelude::GatewayIntents;
use serenity::Result;
use dotenv::dotenv;

#[group]
#[commands(archive)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

const ARCHIVIST_ROLE_ID: u64 = 1170846549870399589;
const ARCHIVED_CHANNEL_PREFIX: &str = "Archived Channels";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&GENERAL_GROUP);

    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(
        token,
        GatewayIntents::non_privileged().union(GatewayIntents::MESSAGE_CONTENT),
    )
    .event_handler(Handler)
    .framework(framework)
    .await
    .expect("Error creating client");

    // start listening for events by starting a single shard
    client.start().await
}

#[command]
async fn archive(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = ctx.http.get_guild(msg.guild_id.unwrap().0).await?;
    if guild.owner_id == msg.author.id
        || is_archivist(&ctx.http, guild.clone(), &msg.author).await?
    {
        let mut channel = ctx
            .http
            .get_channel(msg.channel_id.0)
            .await?
            .guild()
            .unwrap();
        println!(
            "{} is attempting to archive {}.",
            msg.author.name, channel.name
        );
        let guild_channels = ctx.http.get_channels(msg.guild_id.unwrap().0).await?;
        let archive_channels: Vec<&GuildChannel> = guild_channels
            .iter()
            .filter(|&c| c.kind == ChannelType::Category)
            .filter(|&c| c.name.contains(ARCHIVED_CHANNEL_PREFIX))
            .sorted_by(|a, b| a.name.cmp(&b.name))
            .collect();
        let mut destination = (**archive_channels.last().unwrap()).clone();
        let destination_siblings: Vec<&GuildChannel> = guild_channels
            .iter()
            .filter(|&c| c.parent_id.is_some() && c.parent_id.unwrap() == destination.id)
            .collect();
        if destination_siblings.len() > 48 {
            println!("{} is full. Creating a new category.", destination.name());
            destination = guild
                .create_channel(&ctx.http, |c| {
                    c.name(format!(
                        "{} {}",
                        ARCHIVED_CHANNEL_PREFIX,
                        archive_channels.len() + 1
                    ))
                    .kind(ChannelType::Category)
                    .permissions(destination.permission_overwrites.clone())
                })
                .await?;
        }
        let mut name_positions: Vec<(&str, i64)> = destination_siblings
            .iter()
            .map(|&c| (c.name(), c.position))
            .collect();
        name_positions.sort_by_key(|(_, p)| *p);
        let position = (name_positions.last().map(|(_, p)| *p).unwrap_or(1) + 1) as u64;
        channel
            .edit(ctx, |ec| {
                ec.category(destination.id)
                    .position(position)
                    .permissions(destination.permission_overwrites.clone())
            })
            .await?;
        println!("Archiving {} succeeded.", channel.name());
    } else {
        msg.reply(ctx, "You are not an Archivist.").await?;
    }
    Ok(())
}

async fn is_archivist(
    http: impl CacheHttp,
    guild: impl Into<GuildContainer>,
    user: &User,
) -> Result<bool> {
    user.has_role(http, guild, ARCHIVIST_ROLE_ID).await
}
