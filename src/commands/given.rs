use itertools::Itertools;
use serenity::{
    client::Context,
    model::{
        channel::ReactionType,
        id::{EmojiId, UserId},
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue::User,
        },
        prelude::GuildId,
    },
    prelude::Mentionable,
};
use sqlx::{query, query_scalar};

use crate::{
    config::Command,
    data,
    database::client::Database,
    error::KowalskiError,
    utils::{parse_arg_resolved, send_response_complex},
};

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);

    let options = &command.data.options;

    // Parse argument (use command user as fallback)
    let user = if !options.is_empty() {
        match parse_arg_resolved(options, 0)? {
            User(user, ..) => user,
            _ => unreachable!(),
        }
    } else {
        &command.user
    };

    let guild_id = command.guild_id.unwrap().0 as i64;

    // Get guild id
    let user_id = user.id.0 as i64;

    // Analyze reactions from the user
    let (upvotes, downvotes) = {
        let row = query!(
            "
        SELECT SUM(CASE WHEN upvote THEN 1 END) upvotes,
        SUM(CASE WHEN NOT upvote THEN 1 END) downvotes
        FROM score_reactions_v
        WHERE guild=$1 AND user_from = $2;
        ",
            guild_id,
            user_id
        )
        .fetch_one(database.db())
        .await?;

        (
            row.upvotes.unwrap_or_default(),
            row.downvotes.unwrap_or_default(),
        )
    };
    let score = upvotes - downvotes;
    let emojis = {
        let rows = query!(
            "
        SELECT unicode, guild_emoji, emoji_source_guild, COUNT(*) as count FROM score_reactions_v
        WHERE guild = $1 AND user_from = $2
        GROUP BY unicode, guild_emoji, emoji_source_guild
        ORDER BY count DESC
        ",
            guild_id,
            user_id,
        )
        .fetch_all(database.db())
        .await?;

        let mut emojis = Vec::new();

        for row in rows {
            let emoji = match (row.unicode, row.guild_emoji, row.guild) {
                (Some(string), _, _) => ReactionType::Unicode(string),
                (_, Some(id), Some(guild)) => GuildId::from(guild as u64)
                    .emoji(&ctx, EmojiId(id as u64))
                    .await?
                    .into(),
                _ => unreachable!(),
            };

            emojis.push((emoji, row.count.unwrap_or_default()));
        }

        emojis
    };
    let rank = {
        query_scalar!("
            WITH ranks AS (
                SELECT user_from,
                RANK() OVER (
                    ORDER BY COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) DESC, user_from
                ) rank
                FROM score_reactions_v
                WHERE guild = $1
                GROUP BY user_from
            )

            SELECT rank FROM ranks
            WHERE user_from = $2
            ", guild_id, user_id).fetch_optional(database.db()).await?.flatten()
    };
    let rank = match rank {
        Some(rank) => rank.to_string(),
        None => String::from("not available"),
    };

    let top_users: Vec<_> = {
        let rows = query!(
            "
        SELECT user_to, COUNT(*) FILTER (WHERE upvote) upvotes,
        COUNT(*) FILTER (WHERE NOT upvote) downvotes,
        SUM(CASE WHEN upvote THEN 1 ELSE -1 END) FILTER (WHERE NOT native) gifted
        FROM score_reactions_v
        WHERE guild = $1 AND user_from = $2
        GROUP BY user_to
        HAVING COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) >= 0
        ORDER BY COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) DESC
        LIMIT 5
        ",
            guild_id,
            &user_id
        )
        .fetch_all(database.db())
        .await?;

        rows.iter()
            .map(|row| {
                (
                    UserId(row.user_to.unwrap() as u64),
                    row.upvotes.unwrap_or_default(),
                    row.downvotes.unwrap_or_default(),
                    row.gifted.unwrap_or_default(),
                )
            })
            .collect()
    };

    let bottom_users: Vec<_> = {
        let rows = query!(
            "
        SELECT user_to, COUNT(*) FILTER (WHERE upvote) upvotes,
        COUNT(*) FILTER (WHERE NOT upvote) downvotes,
        SUM(CASE WHEN upvote THEN 1 ELSE -1 END) FILTER (WHERE NOT native) gifted
        FROM score_reactions_v
        WHERE guild = $1 AND user_from = $2
        GROUP BY user_to
        HAVING COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) < 0
        ORDER BY COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) ASC
        LIMIT 5
        ",
            guild_id,
            user_id,
        )
        .fetch_all(database.db())
        .await?;

        rows.iter()
            .map(|row| {
                (
                    UserId(row.user_to.unwrap() as u64),
                    row.upvotes.unwrap_or_default(),
                    row.downvotes.unwrap_or_default(),
                    row.gifted.unwrap_or_default(),
                )
            })
            .collect()
    };

    send_response_complex(
        &ctx,
        &command,
        command_config,
        &format!("Score given out by {}", user.name),
        &format!(
            "The user {} has given out a total score of **{}** [+{}, -{}] (rank **{}**).",
            user.mention(),
            score,
            upvotes,
            downvotes,
            rank
        ),
        |embed| {
            let mut emojis = emojis
                .iter()
                .map(|(reaction, count)| {
                    let f_count = *count as f64;
                    let f_total = (upvotes + downvotes) as f64;
                    format!(
                        "**{}x{}** ({:.1}%)",
                        count,
                        reaction.to_string(),
                        f_count / f_total * 100f64
                    )
                })
                .join(", ");
            if emojis.is_empty() {
                emojis = "Not available".to_string();
            }

            let mut top_users = top_users
                .iter()
                .map(|(user, upvotes, downvotes, gifted)| {
                    format!(
                        "{}: **{}** [+{}, -{}] ({} gifted)",
                        user.mention(),
                        upvotes - downvotes,
                        upvotes,
                        downvotes,
                        gifted
                    )
                })
                .join("\n");
            if top_users.is_empty() {
                top_users = "Not available".to_string();
            }

            let mut bottom_users = bottom_users
                .iter()
                .map(|(user, upvotes, downvotes, gifted)| {
                    format!(
                        "{}: **{}** [+{}, -{}] ({} gifted)",
                        user.mention(),
                        upvotes - downvotes,
                        upvotes,
                        downvotes,
                        gifted
                    )
                })
                .join("\n");
            if bottom_users.is_empty() {
                bottom_users = "Not available".to_string();
            }

            embed.fields(vec![
                ("Favorite emojis", emojis, false),
                ("Top 5 upvoted", top_users, false),
                ("Top 5 downvoted", bottom_users, false),
            ])
        },
        Vec::new(),
    )
    .await
}
