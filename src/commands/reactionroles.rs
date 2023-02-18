use itertools::Itertools;
use serenity::{
    client::Context,
    model::{
        channel::ReactionType,
        id::{ChannelId, EmojiId, MessageId, RoleId},
        interactions::application_command::ApplicationCommandInteraction, prelude::GuildId,
    },
    prelude::Mentionable, futures::TryStreamExt,
};
use sqlx::query;

use crate::{
    config::Command, data, database::client::Database, error::KowalskiError, pluralize,
    utils::send_response,
};

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);

    let guild_id = command.guild_id.unwrap();

    // Get guild id
    let guild_db_id = guild_id.0 as i64;

    // Get reaction roles
    let roles:Vec<_> =query!(
                "
                SELECT c.channel, m.message, e.unicode, e.guild_emoji, e.guild as guild_emoji_guild, r.role, rr.slots
                FROM reaction_roles rr
                INNER JOIN emojis e ON rr.emoji = e.id
                INNER JOIN messages m ON rr.message = m.id
                INNER JOIN channels c ON m.channel = c.id
                INNER JOIN roles r ON rr.role = r.id
                WHERE c.guild = $1
                ORDER BY channel, message
                ",
                guild_db_id,
            ).fetch(database.db()).map_err(|e|e.into())
            .and_then(|row|async move{Ok::<_,KowalskiError>((
                ChannelId(row.channel as u64),
                MessageId(row.message as u64),
                match(row.unicode,row.guild_emoji,row.guild_emoji_guild){
                    (Some(string),..)=>ReactionType::Unicode(string),
                    (_,Some(id),Some(guild))=>GuildId(guild as u64)
                        .emoji(ctx, EmojiId(id as u64)).await?.into(),
                    _=>unreachable!(),
                },
                RoleId(row.role as u64),
                row.slots
            ))}).try_collect()
            .await?;



    let roles = roles
        .iter()
        .map(|(channel_id, message_id, emoji, role_id, slots)| {
            let mut content = format!(
                "{} when reacting with {} [here]({}).",
                role_id.mention(),
                emoji.to_string(),
                message_id.link(*channel_id, Some(guild_id))
            );

            if let Some(slots) = slots {
                content.push_str(&format!(
                    " (There {} currently {} available)",
                    if *slots == 1 { "is" } else { "are" },
                    pluralize!("slot", *slots)
                ));
            }

            content
        })
        .join("\n");

    let title = "Reaction roles";

    if roles.is_empty() {
        send_response(
            ctx,
            command,
            command_config,
            title,
            "There are no reaction roles registered on this guild.",
        )
        .await
    } else {
        send_response(
            ctx,
            command,
            command_config,
            title,
            &format!(
                "The following reaction roles are registered on this guild:\n\n{}",
                roles
            ),
        )
        .await
    }
}
