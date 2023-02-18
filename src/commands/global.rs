use itertools::Itertools;
use serenity::{
    client::Context,
    model::{
        channel::ReactionType,
        id::EmojiId,
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
    pluralize,
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

    let user_id = user.id.0 as i64;

    // Count active guilds of the user
    let guilds = query_scalar!(
        "
        SELECT COUNT(*) guilds
        FROM users
        WHERE \"user\" = $1::BIGINT
        ",
        user_id
    )
    .fetch_one(database.db())
    .await?
    .unwrap_or_default();

    // Analyze reactions of the user
    let (upvotes, downvotes) = {
        let row = query!(
            "
        SELECT SUM(CASE WHEN upvote THEN 1 END) upvotes,
        SUM(CASE WHEN NOT upvote THEN 1 END) downvotes
        FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji = se.id
        INNER JOIN users u ON u.id = r.user_to
        WHERE u.user = $1
        ",
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
        SELECT unicode, guild_emoji,e.guild, COUNT(*)
        FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji=se.id
        INNER JOIN emojis e ON se.emoji = e.id
        INNER JOIN users u ON u.id = r.user_to
        WHERE u.user = $1
        GROUP BY unicode, guild_emoji, e.guild
        ORDER BY count DESC
        ",
            user_id
        )
        .fetch_all(database.db())
        .await?;

        let mut emojis = Vec::new();

        for row in rows {
            let emoji = match (row.unicode, row.guild_emoji, row.guild) {
                (Some(string), _, _) => ReactionType::Unicode(string),
                (_, Some(id), Some(guild)) => GuildId::from(guild as u64)
                    .emoji(&ctx.http, EmojiId(id as u64))
                    .await?
                    .into(),

                _ => unreachable!(),
            };

            emojis.push((emoji, row.count.unwrap_or_default()));
        }

        emojis
    };
    // Get rank of the user
    let rank = query_scalar!(r#"
            WITH ranks AS (
                SELECT u.user,
                RANK() OVER (
                    ORDER BY COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) DESC, u.user
                ) rank
                FROM score_reactions r
                INNER JOIN score_emojis se ON r.emoji = se.id
                INNER JOIN users u ON r.user_to=u.id
                GROUP BY u.user
            )

            SELECT rank FROM ranks
            WHERE "user" = $1;
            "#,
                             user_id
    ).fetch_optional(database.db()).await?.flatten();
    let rank = match rank {
        Some(rank) => rank.to_string(),
        None => String::from("not available"),
    };

    let (given_upvotes, given_downvotes) = {
        let row = query!(
            "
        SELECT SUM(CASE WHEN upvote THEN 1 END) upvotes,
        SUM(CASE WHEN NOT upvote THEN 1 END) downvotes
        FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji = se.id
        WHERE user_from = $1
        ",
            user_id,
        )
        .fetch_one(database.db())
        .await?;

        (
            row.upvotes.unwrap_or_default(),
            row.downvotes.unwrap_or_default(),
        )
    };
    let given = given_upvotes - given_downvotes;
    let given_emojis = {
        let rows = query!(
            "
        SELECT unicode, guild_emoji,e.guild, COUNT(*)
        FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji=se.id
        INNER JOIN emojis e ON se.emoji = e.id
        WHERE user_from = $1
        GROUP BY unicode, guild_emoji,e.guild
        ORDER BY count DESC
        ",
            user_id,
        )
        .fetch_all(database.db())
        .await?;

        let mut emojis = Vec::new();

        for row in rows {
            let emoji = match (row.unicode, row.guild_emoji, row.guild) {
                (Some(string), _, _) => ReactionType::Unicode(string),
                (_, Some(id), Some(guild)) => GuildId::from(guild as u64)
                    .emoji(&ctx.http, EmojiId(id as u64))
                    .await?
                    .into(),

                _ => unreachable!(),
            };

            emojis.push((emoji, row.count.unwrap_or_default()));
        }

        emojis
    };
    let given_rank = query_scalar!("
            WITH ranks AS (
                SELECT user_from,
                RANK() OVER (
                    ORDER BY COUNT(*) FILTER (WHERE upvote) - COUNT(*) FILTER (WHERE NOT upvote) DESC, user_from
                ) rank
                FROM score_reactions r
                INNER JOIN score_emojis se ON r.emoji = se.emoji
                GROUP BY user_from
            )

            SELECT rank FROM ranks
            WHERE user_from = $1::BIGINT
            ", user_id).fetch_optional(database.db()).await?.flatten();

    let given_rank = match given_rank {
        Some(given_rank) => given_rank.to_string(),
        None => String::from("not available"),
    };

    send_response_complex(
        &ctx,
        &command,
        command_config,
        &format!("Global stats of {}", user.name),
        &format!(
            "The user {} is currently active on {} shared with the bot.",
            user.mention(),
            pluralize!("guild", guilds)
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

            let mut given_emojis = given_emojis
                .iter()
                .map(|(reaction, count)| {
                    let f_count = *count as f64;
                    let f_total = (given_upvotes + given_downvotes) as f64;
                    format!(
                        "**{}x{}** ({:.1}%)",
                        count,
                        reaction.to_string(),
                        f_count / f_total * 100f64
                    )
                })
                .join(", ");
            if given_emojis.is_empty() {
                given_emojis = "Not available".to_string();
            }

            embed.fields(vec![
                (
                    "Score",
                    &format!(
                        "The user has a global score of **{}** [+{}, -{}] (rank **{}**).",
                        score, upvotes, downvotes, rank
                    ),
                    false,
                ),
                ("The following emojis were used", &emojis, false),
                (
                    "Given",
                    &format!(
                        "The user has given out a global score of **{}** [+{}, -{}] (rank **{}**).",
                        given, given_upvotes, given_downvotes, given_rank
                    ),
                    false,
                ),
                ("The following emojis were used", &given_emojis, false),
            ])
        },
        Vec::new(),
    )
    .await
}
