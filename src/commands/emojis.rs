use itertools::Itertools;
use serenity::{
    client::Context,
    model::{
        channel::ReactionType, id::EmojiId,
        interactions::application_command::ApplicationCommandInteraction, prelude::GuildId,
    },
};
use sqlx::query;

use crate::{
    config::Command, data, database::client::Database, error::KowalskiError, utils::send_response,
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
    let guild_db_id = database.get_guild(guild_id).await?;

    // Get up- and downvote emojis
    let (upvotes, downvotes) = {
        let rows = query!(
                "
                SELECT emojis.unicode, emojis.guild_emoji, emojis.guild, score_emojis.upvote FROM score_emojis
                INNER JOIN emojis  ON score_emojis.emoji = emojis.id
                WHERE score_emojis.guild = $1
                ",
                guild_db_id,
            ).fetch_all(database.db())
            .await?;

        let mut upvotes = Vec::new();
        let mut downvotes = Vec::new();

        for row in rows {
            let emoji = match (row.unicode, row.guild_emoji, row.guild) {
                (Some(string), _, _) => ReactionType::Unicode(string),
                (_, Some(id), Some(guild)) => GuildId::from(guild as u64)
                    .emoji(&ctx.http, EmojiId(id as u64))
                    .await?
                    .into(),
                _ => unreachable!(),
            };

            if row.upvote {
                upvotes.push(emoji);
            } else {
                downvotes.push(emoji);
            }
        }

        (upvotes, downvotes)
    };

    let title = "Reaction emojis";

    if upvotes.is_empty() && downvotes.is_empty() {
        send_response(
            ctx,
            command,
            command_config,
            title,
            "There are no reaction emojis registered on this guild.",
        )
        .await
    } else {
        let mut content =
            "The following reaction emojis are registered on this guild:\n\n".to_string();

        if !upvotes.is_empty() {
            content.push_str(&format!(
                "**Upvotes:** {}\n",
                upvotes.iter().map(|emoji| emoji.to_string()).join(", ")
            ));
        }

        if !downvotes.is_empty() {
            content.push_str(&format!(
                "**Downvotes:** {}\n",
                downvotes.iter().map(|emoji| emoji.to_string()).join(", ")
            ));
        }

        send_response(ctx, command, command_config, title, &content).await
    }
}
