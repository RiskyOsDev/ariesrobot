use std::{error::Error as ErrorTrait, fmt::Display};

use poise::{
    serenity_prelude::{self as serenity, GatewayIntents, GuildId},
    PrefixFrameworkOptions,
};
use sqlx::{postgres::PgConnectOptions, Pool, Postgres};

struct Data {
    db: Pool<Postgres>
} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn ErrorTrait + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug, PartialEq, Eq)]
pub enum BotError {
    NoPermission
}

unsafe impl Send for BotError {}
unsafe impl Sync for BotError {}

impl Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ErrorTrait for BotError {}

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn ping(
    ctx: Context<'_>,
    #[description = "text to return"] string: Option<String>,
) -> Result<(), Error> {
    match string {
        None => ctx.say("pong").await?,
        Some(string) => ctx.say(format!("pong: {}", string)).await?,
    };
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn gid(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say(format!("guild id: {:?}", ctx.guild_id())).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn get_user(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get() as i64;

    let get_user = sqlx::query!("SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(&ctx.data().db);
        
    ctx.say(format!("{:?}", get_user.await)).await?;

    return Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn add_user(
    ctx: Context<'_>,
    #[description = "user to add"]
    user: Option<serenity::User>
) -> Result<(), Error> {
    let user = user.unwrap_or_else(|| ctx.author().clone() );
    let user_id = user.id.get() as i64;
    let user_string = user.name.clone();

    let res = sqlx::query!("INSERT INTO users (id, name) VALUES ($1, $2)", user_id, user_string).execute(&ctx.data().db).await;

    match res {
        Ok(_) => {ctx.say(format!("user {} was created", user_string)).await?;},
        Err(e) => {ctx.say(format!("failed to create user because: {:?}", e)).await?;}
    }

    return Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn rm_user(
    ctx: Context<'_>, 
    #[description = "user to remove"]
    user: Option<serenity::User>
) -> Result<(), Error> {
    let r_id = ctx.guild().expect("no guild").role_by_name("bot_admin").expect("no role").id;
    // check that caller has bot_admin permission if we use another user
    if !(ctx.author().has_role(ctx, ctx.guild_id().ok_or(BotError::NoPermission)?, r_id).await?) && user.is_some() {
        ctx.say("need admin permission to remove other user").await?;
        return Ok(());
    }
    let user = user.unwrap_or_else(|| ctx.author().clone() );
    let user_id = user.id.get() as i64;
    let user_string = user.name.clone();

    if sqlx::query!("SELECT * FROM users WHERE id = $1", user_id).fetch_one(&ctx.data().db).await.is_err() {
        ctx.say(format!("user {} doesn't exixt", user_string)).await?;
        return Ok(())
    }

    match sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&ctx.data().db).await {
        Ok(_) => {ctx.say(format!("user {} was deleted", user_string)).await?;},
        Err(_e) => {ctx.say(format!("failed to delete user")).await?;}
    }

    return Ok(())
}

#[tokio::main]
async fn main() {
    // start twitch data gatherer
    let _tw_token = {
        use std::io::Read;
        let mut buf = String::new();
        std::fs::File::open("twitch.token")
            .expect("couldn't open twitch.token to read discord bot token")
            .read_to_string(&mut buf)
            .expect("couldn't read twitch.token");
        buf.trim().to_owned()
    };

    // start discord bot
    let ds_token = {
        use std::io::Read;
        let mut buf = String::new();
        std::fs::File::open("bot.token")
            .expect("couldn't open bot.token to read discord bot token")
            .read_to_string(&mut buf)
            .expect("couldn't read bot.token");
        buf.trim().to_owned()
    };
    let intents = GatewayIntents::non_privileged().union(GatewayIntents::MESSAGE_CONTENT);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![age(), ping(), gid(), get_user(), add_user(), rm_user()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("!".into()),
                case_insensitive_commands: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    GuildId::new(1290689349897162803),
                )
                .await?;
                Ok(Data {
                    db: Pool::connect_with(PgConnectOptions::new().database("aries").host("localhost").username("postgres").password("example")).await?
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(ds_token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
