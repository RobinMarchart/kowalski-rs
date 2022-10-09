use std::time::Duration;

use itertools::Itertools;
use log::error;
use serenity::{futures::{StreamExt, stream, TryStreamExt}, client::Context, model::{
        interactions::application_command::ApplicationCommandInteraction,
        prelude::{ChannelId, EmojiId, GuildId, MessageId, ReactionType},
    }};
use sqlx::{query, query_scalar};
use tracing::info;

use crate::{
    config::{Command, Config},
    data,
    database::client::Database,
    error::KowalskiError,
    utils::{send_confirmation, send_response, InteractionResponse},
};

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), KowalskiError> {
    // Get config and database
    let (config, database) = data!(ctx, (Config, Database));

    let title = "Clean database tables";

    // Check for the interaction response
    let response = send_confirmation(
        ctx,
        command,
        command_config,
        "Are you really sure you want to clean the database tables?
            This cannot be reversed!",
        Duration::from_secs(config.general.interaction_timeout),
    )
    .await?;

    match response {
        Some(InteractionResponse::Continue) => {
            // Clean all the database tables
            match clean_database(ctx, &database).await {
                Ok(_) => {
                    send_response(
                        &ctx,
                        &command,
                        command_config,
                        &title,
                        "I successfully cleaned all tables.
                Please make sure no data was lost.",
                    )
                    .await
                }
                Err(e) => {
                    error!("Error occurred during clean: {}", &e);
                    send_response(
                        ctx,
                        command,
                        command_config,
                        title,
                        "Aborted the action due to an internal error.",
                    )
                    .await?;
                    Err(e)
                }
            }
        }
        Some(InteractionResponse::Abort) => {
            send_response(ctx, command, command_config, title, "Aborted the action.").await
        }
        None => Ok(()),
    }
}

async fn clean_database(ctx: &Context, db: &Database) -> Result<(), KowalskiError> {
    // Get database
    let ctx = ctx;
    let transaction = db.begin().await?;
    let db = db.db();

    info!("cleaning database");

    let deleted = query!(
        r#"DELETE FROM guilds WHERE NOT guild=ANY($1)"#,
        &ctx.cache
            .guilds()
            .into_iter()
            .map(|g| g.0 as i64)
            .collect_vec()[..]
    )
    .execute(db)
    .await?
    .rows_affected();
    info!("{} guild(s) deleted", deleted);

    for guild_id in query_scalar!("SELECT guild FROM guilds;")
        .fetch_all(db)
        .await?
    {
        let discord_guild_id = GuildId::from(guild_id as u64);

        info!(
            "cleaning guild {}",
            discord_guild_id
                .name(ctx)
                .unwrap_or_else(|| "unknown".to_string())
        );
        let members: Vec<i64> = discord_guild_id
            .members_iter(ctx)
            .map_ok(|member| member.user.id.0 as i64)
            .try_collect()
            .await?;
        let deleted = query!(
            r#"DELETE FROM users WHERE guild=$1 AND NOT "user"=ANY($2)"#,
            guild_id,
            &members[..]
        )
        .execute(db)
        .await?
        .rows_affected();
        info!("{} user(s) deleted", deleted);
        let roles = discord_guild_id
            .roles(ctx)
            .await?
            .into_iter()
            .map(|r| r.0 .0 as i64)
            .collect_vec();
        let deleted = query!(
            "DELETE FROM roles WHERE guild=$1 AND NOT role=ANY($2)",
            guild_id,
            &roles[..]
        )
        .execute(db)
        .await?
        .rows_affected();
        info!("{} role(s) deleted", deleted);
        let emojis = discord_guild_id
            .emojis(ctx)
            .await?
            .into_iter()
            .map(|emoji| emoji.id.0 as i64)
            .collect_vec();
        let deleted = query!(
            "DELETE FROM emojis WHERE guild=$1 AND NOT guild_emoji=ANY($2)",
            guild_id,
            &emojis[..]
        )
        .execute(db)
        .await?
        .rows_affected();
        info!("{} emoji(s) deleted", deleted);
        let channels = discord_guild_id
            .channels(ctx)
            .await?
            .into_iter()
            .map(|channel| channel.0 .0 as i64)
            .collect_vec();
        let deleted = query!(
            "DELETE FROM channels WHERE guild=$1 AND NOT channel=ANY($2)",
            guild_id,
            &channels[..]
        )
        .execute(db)
        .await?
        .rows_affected();
        info!("{} channel(s) deleted", deleted);
        let mut deleted_messages = 0u64;
        let mut deleted_reactions = 0u64;
        for message in query!("SELECT messages.id as id,messages.message as message,channels.channel as channel FROM messages INNER JOIN channels ON messages.channel=channels.id WHERE channels.guild=$1",guild_id)
            .fetch_all(db).await?{
                let channel_id=ChannelId::from(message.channel as u64);
                let message_id=MessageId::from(message.message as u64);
                match channel_id.message(ctx, message_id).await {
                    Ok(discord_message) => {
                        for emoji in query!(
                            "SELECT
score_emojis.id as id,
emojis.guild as guild,
emojis.guild_emoji as guild_emoji,
emojis.unicode as unicode
FROM emojis INNER JOIN score_emojis ON score_emojis.emoji=emojis.id
WHERE $1 IN (SELECT message FROM score_reactions WHERE emoji=score_emojis.id)",message.id)
.fetch_all(db).await?{
    let reaction=match &emoji.guild {
        Some(guild) => GuildId::from(*guild as u64).emoji(ctx, EmojiId::from((emoji.guild_emoji.unwrap()) as u64)).await?.into(),
        None => ReactionType::Unicode(emoji.unicode.unwrap().clone()),
    };
    let discord_message=&discord_message;
    let reaction=&reaction;
    let users_from:Vec<i64>=stream::unfold(Some(None), |after|
                   {
                       async move{
        if let Some(after)=after{
    match discord_message.reaction_users(ctx, reaction.to_owned(), Some(100), after).await{
        Ok(users) => {
            let after=if users.len()<100{Some(Some(users[99].id.clone()))}else{None};
            Some((Ok(users),after))
        },
        Err(e) => Some((Err(e),None)),
    }
        }else{None}
    }})
        .map_ok(|users|stream::iter(users.into_iter().map(|user| -> Result<i64, serenity::Error> {Ok(user.id.0 as i64)})))
       .try_flatten().try_collect().await?;
    deleted_reactions+= query!("DELETE FROM score_reactions WHERE message=$1 AND channel=$2 AND emoji=$3 AND NOT user_from=ANY($4);",
           message.message,message.channel,emoji.id,&users_from[..]
).execute(db).await?.rows_affected();
}
                    }
                    Err(serenity::Error::Http(_)) => {
                        query!("DELETE FROM messages WHERE id=$1",message.id).execute(db).await?;
                        deleted_messages+=1;
                    }
                    Err(e)=>return Err(e.into())
                }
            }
        info!("{} messages(s) deleted", deleted_messages);
        info!("{} reactions(s) deleted", deleted_reactions);
    }
    transaction.commit().await?;
    Ok(())
}
