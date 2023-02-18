use itertools::Itertools;
use serenity::{
    client::Context,
    futures::TryStreamExt,
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

    // Get guild
    let guild_id = command.guild_id.unwrap();

    // Get guild and user ids
    let guild_db_id = database.get_guild(guild_id).await?;
    let user_db_id = database.get_user(guild_id, user.id).await?;

    // Analyze reactions of the user
    let (upvotes, downvotes) = {
        let row = query!(
            "
        SELECT SUM(CASE WHEN upvote THEN 1 END) upvotes,
        SUM(CASE WHEN NOT upvote THEN 1 END) downvotes
        FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji = se.id
        WHERE user_to = $1
        ",
            user_db_id
        )
        .fetch_one(database.db())
        .await?;

        (
            row.upvotes.unwrap_or_default(),
            row.downvotes.unwrap_or_default(),
        )
    };
    let score = upvotes - downvotes;
    let emojis = query!(
        "
        SELECT e.unicode, e.guild_emoji,e.guild, COUNT(*) FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji = se.id
        INNER JOIN emojis e ON se.emoji = e.id
        WHERE  r.user_to = $1
        GROUP BY e.id
        ORDER BY count DESC
        ",
        user_db_id
    )
    .fetch(database.db())
    .map_err(|e| e.into())
    .and_then(|row| async move {
        Ok::<_, KowalskiError>((
            match (row.unicode, row.guild_emoji, row.guild) {
                (Some(string), ..) => ReactionType::Unicode(string),
                (_, Some(id), Some(guild)) => GuildId(guild as u64)
                    .emoji(ctx, EmojiId(id as u64))
                    .await?
                    .into(),
                _ => unreachable!(),
            },
            row.count.unwrap_or_default(),
        ))
    })
    .try_collect()
    .await?;

    let rank =  query_scalar!("
            WITH ranks AS (
                SELECT user_to,
                RANK() OVER (
                    ORDER BY COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) DESC, user_to
                ) rank
                FROM score_reactions r
                INNER JOIN score_emojis se ON r.emoji = se.id
                INNER JOIN users u ON r.user_to=u.id
                WHERE u.guild = $1
                GROUP BY user_to
            )

            SELECT rank FROM ranks
            WHERE user_to = $2;
            ", guild_db_id, user_db_id).fetch_optional(database.db()).await?.flatten();

    let rank = match rank {
        Some(rank) => rank.to_string(),
        None => String::from("not available"),
    };

    let top_users: Vec<_> = query!(
        "
        SELECT user_from, COUNT(*) FILTER (WHERE upvote) upvotes,
        COUNT(*) FILTER (WHERE NOT upvote) downvotes
        FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji = se.id
        WHERE r.user_to = $1 AND native = true
        GROUP BY user_from
        HAVING COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) >= 0
        ORDER BY COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) DESC
        LIMIT 5
        ",
        user_db_id,
    )
    .fetch(database.db())
    .map_ok(|row| {
        (
            UserId(row.user_from as u64),
            row.upvotes.unwrap_or_default(),
            row.downvotes.unwrap_or_default(),
        )
    })
    .try_collect()
    .await?;

    let bottom_users: Vec<_> = query!(
        "
        SELECT user_from, COUNT(*) FILTER (WHERE upvote) upvotes,
        COUNT(*) FILTER (WHERE NOT upvote) downvotes
        FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji = se.id
        INNER JOIN users u ON r.user_to = u.id
        WHERE r.user_to = $1 AND native = true
        GROUP BY user_from
        HAVING COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) < 0
        ORDER BY COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) ASC
        LIMIT 5
        ",
        user_db_id,
    )
    .fetch(database.db())
    .map_ok(|row| {
        (
            UserId(row.user_from as u64),
            row.upvotes.unwrap_or_default(),
            row.downvotes.unwrap_or_default(),
        )
    })
    .try_collect()
    .await?;

    send_response_complex(
        &ctx,
        &command,
        command_config,
        &format!("Score of {}", user.name),
        &format!(
            "The user {} currently has a score of **{}** [+{}, -{}] (rank **{}**).",
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
                .map(|(user, upvotes, downvotes)| {
                    format!(
                        "{}: **{}** [+{}, -{}]",
                        user.mention(),
                        upvotes - downvotes,
                        upvotes,
                        downvotes
                    )
                })
                .join("\n");
            if top_users.is_empty() {
                top_users = "Not available".to_string();
            }

            let mut bottom_users = bottom_users
                .iter()
                .map(|(user, upvotes, downvotes)| {
                    format!(
                        "{}: **{}** [+{}, -{}]",
                        user.mention(),
                        upvotes - downvotes,
                        upvotes,
                        downvotes
                    )
                })
                .join("\n");
            if bottom_users.is_empty() {
                bottom_users = "Not available".to_string();
            }

            embed.fields(vec![
                ("Emojis", emojis, false),
                ("Top 5 benefactors", top_users, false),
                ("Top 5 haters", bottom_users, false),
            ])
        },
        Vec::new(),
    )
    .await
}
