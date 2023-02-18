use serenity::{
    client::Context,
    model::{
        channel::{Message, Reaction, ReactionType},
        guild::Member,
        id::{ChannelId, GuildId, MessageId, RoleId, UserId},
    },
};
use sqlx::{query, query_as, query_scalar};

use crate::{
    config::Config,
    cooldowns::Cooldowns,
    data,
    database::{client::Database, types::Modules},
    error::KowalskiError,
};

pub async fn reaction_add(ctx: &Context, add_reaction: Reaction) -> Result<(), KowalskiError> {
    // Get database
    let (config, database, cooldowns_lock) = data!(ctx, (Config, Database, Cooldowns));

    // Check if the emoji is registered and get its id
    if let Some(emoji_id) = get_emoji_id(&add_reaction.emoji, &database).await? {
        // Get reaction data
        let (guild_id, user_from_id, user_to_id, channel_id, message_id) =
            get_reaction_data(ctx, &add_reaction).await?;

        // Get guild status
        let status = query_as!(
            Modules,
            r#"
            SELECT owner, utility, score, reaction_roles, "analyze"
            FROM modules
            WHERE guild = $1
            "#,
            guild_id,
        )
        .fetch_optional(database.db())
        .await?
        .unwrap_or_default();

        // Get the reaction-roles to assign
        let reaction_roles: Vec<_> = if status.reaction_roles {
            query!(
                "
                    SELECT role, slots, _reaction_roles_id FROM reaction_roles_v
                    WHERE guild = $1 AND channel = $2 AND message = $3
                    AND _emoji_id = $4
                    ",
                guild_id,
                channel_id,
                message_id,
                emoji_id
            )
            .fetch_all(database.db())
            .await?
        } else {
            Vec::new()
        };

        // Whether the emoji should count as a up-/downvote
        let (levelup, score_emoji_id) =
            if status.score && user_from_id != user_to_id && reaction_roles.is_empty() {
                query!(
                    "SELECT upvote,id FROM score_emojis
                     WHERE guild = $1 AND emoji = $2
                    ",
                    guild_id,
                    emoji_id
                )
                .fetch_optional(database.db())
                .await?
                .map(|row| {
                    if row.upvote {
                        (1, row.id)
                    } else {
                        (-1, row.id)
                    }
                })
                .unwrap_or_default()
            } else {
                (0, 0)
            };

        if !reaction_roles.is_empty() {
            // Get guild
            let guild_id = GuildId::from(guild_id as u64);
            // Get the member
            let mut member = guild_id.member(&ctx, user_from_id as u64).await?;

            // Never give roles to bots
            if member.user.bot {
                return Ok(());
            }

            // Remove the reaction
            add_reaction.delete(&ctx.http).await?;

            for row in reaction_roles {
                let (role, slots, id) = (
                    row.role.unwrap(),
                    row.slots,
                    row._reaction_roles_id.unwrap(),
                );
                if member
                    .roles
                    .contains(&RoleId::from(row.role.unwrap() as u64))
                {
                    // Remove role from user
                    member
                        .remove_role(&ctx.http, RoleId::from(role as u64))
                        .await?;
                    if slots.is_some() {
                        // Increment slots
                        query!(
                            "
                        UPDATE reaction_roles
                        SET slots = slots + 1
                        WHERE id = $1 AND slots IS NOT NULL
                        ",
                            id
                        )
                        .execute(database.db())
                        .await?;
                    }
                } else if slots.is_none() {
                    member
                        .add_role(&ctx.http, RoleId::from(role as u64))
                        .await?;
                } else if slots.unwrap() > 0 {
                    //prevent unsuccessful member add from still increasing the slot count
                    let mut transaction = database.begin().await?;
                    query!(
                        "
                        UPDATE reaction_roles
                        SET slots = slots - 1
                        WHERE id = $1 AND slots IS NOT NULL
                        ",
                        id
                    )
                    .execute(&mut transaction)
                    .await?;
                    // Add role to user
                    member
                        .add_role(&ctx.http, RoleId::from(role as u64))
                        .await?;
                    transaction.commit().await?;
                }
            }
        } else if levelup != 0 {
            // Check for cooldown
            let cooldown_active = {
                let mut cooldowns = cooldowns_lock.write().await;

                // Get role ids of user
                let roles: Vec<_> = add_reaction
                    .member
                    .as_ref()
                    .unwrap()
                    .roles
                    .iter()
                    .map(|role_id| role_id.clone())
                    .collect();

                cooldowns
                    .check_cooldown(&config, &database, guild_id, user_from_id, &roles)
                    .await?
            };

            if cooldown_active {
                // Remove reaction
                add_reaction.delete(&ctx.http).await?;
            } else {
                let user = database
                    .get_user(GuildId(guild_id as u64), UserId(user_to_id as u64))
                    .await?;

                query!(
                    "INSERT INTO score_reactions (user_from,user_to,channel,message,emoji)
                        VALUES ($1, $2, $3, $4, $5 )",
                    user_from_id,
                    user,
                    channel_id,
                    message_id,
                    score_emoji_id
                )
                .execute(database.db())
                .await?;

                // Get guild
                let guild_id = GuildId::from(guild_id as u64);
                // Get the member
                let mut member = guild_id.member(&ctx, user_to_id as u64).await?;
                // Update the roles of the user
                update_roles(&ctx, &database, &mut member).await?;

                // Auto moderate the message if necessary
                let message = add_reaction.message(&ctx.http).await?;
                auto_moderate(&ctx, &database, guild_id, message).await?;
            }
        }
    }

    Ok(())
}

pub async fn reaction_remove(
    ctx: &Context,
    removed_reaction: Reaction,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);
    let (guild_id, user_from_id, user_to_id, channel_id, message_id) =
        get_reaction_data(ctx, &removed_reaction).await?;

    // Get guild status
    let status = query_as!(
        Modules,
        r#"
            SELECT owner, utility, score, reaction_roles, "analyze"
            FROM modules
            WHERE guild = $1
            "#,
        guild_id,
    )
    .fetch_optional(database.db())
    .await?
    .unwrap_or_default();

    //removing score is only necessary if score is actively tracked;
    if status.score {
        // Check if the emoji is registered
        if let Some(emoji_db_id) = match &removed_reaction.emoji {
            ReactionType::Unicode(string) => {
                query_scalar!(
                    "
                SELECT _score_emoji_id FROM score_emojis_v
                WHERE unicode = $1 AND guild = $2
                ",
                    string,
                    guild_id
                )
                .fetch_optional(database.db())
                .await?
            }
            ReactionType::Custom { id: emoji_id, .. } => {
                query_scalar!(
                    "
                    SELECT _score_emoji_id FROM score_emojis_v
                    WHERE guild_emoji = $1 AND guild = $2
                    ",
                    emoji_id.0 as i64,
                    guild_id
                )
                .fetch_optional(database.db())
                .await?
            }
            _ => unreachable!(),
        }
        .flatten()
        {
            // Get reaction data

            if let Some(user_to_db_id) = query_scalar!(
                "DELETE FROM score_reactions
                    USING users AS u
                    WHERE u.guild=$1 AND u.user=$2 AND user_to=u.id AND user_from=$3 AND channel=$4 AND message=$5 AND emoji=$6
                    RETURNING u.user",
            guild_id,user_to_id,user_from_id,channel_id,message_id,emoji_db_id).fetch_optional(database.db()).await?{
// Get guild
                let guild_id = GuildId::from(guild_id as u64);
                // Get the member
                let mut member = guild_id.member(&ctx, user_to_db_id as u64).await?;

            // Update the roles of the user
            update_roles(&ctx, &database, &mut member).await?;

            // Auto moderate the message if necessary
            let message = removed_reaction.message(&ctx.http).await?;
            auto_moderate(&ctx, &database, guild_id, message).await?;}
        }
    }

    Ok(())
}

pub async fn reaction_remove_all(
    ctx: Context,
    channel_id: ChannelId,
    removed_from_message_id: MessageId,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);

    if let Some(guild_id) = {
        let channel = channel_id.to_channel(&ctx.http).await?;
        channel.guild().map(|channel| channel.guild_id.0 as i64)
    } {
        let message_id = removed_from_message_id.0 as i64;
        let channel_id = channel_id.0 as i64;

        let affected_users = query_scalar!(
            "WITH r AS (DELETE FROM score_reactions
                USING users AS u
                WHERE u.guild=$1 and channel=$2 and message=$3 and user_to=u.id
                RETURNING u.user)
            SELECT DISTINCT user FROM r",
            guild_id,channel_id,message_id
        ).fetch_all(database.db()).await?;
        for user in affected_users{
             let guild_id = GuildId::from(guild_id as u64);
                // Get the member
                let mut member = guild_id.member(&ctx, user as u64).await?;

            // Update the roles of the user
            update_roles(&ctx, &database, &mut member).await?
        }
    }

    Ok(())
}

async fn get_emoji_id(
    emoji: &ReactionType,
    database: &Database,
) -> Result<Option<i64>, KowalskiError> {
    let result = match emoji {
        ReactionType::Unicode(string) => {
            query_scalar!(
                "
                SELECT id FROM emojis
                WHERE unicode = $1
                ",
                string,
            )
            .fetch_optional(database.db())
            .await?
        }
        ReactionType::Custom { id: emoji_id, .. } => {
            query_scalar!(
                "
                    SELECT id FROM emojis
                    WHERE guild_emoji = $1
                    ",
                emoji_id.0 as i64
            )
            .fetch_optional(database.db())
            .await?
        }
        _ => unreachable!(),
    };
    Ok(result)
}

async fn get_reaction_data(
    ctx: &Context,
    reaction: &Reaction,
) -> Result<(i64, i64, i64, i64, i64), KowalskiError> {
    let guild_id = reaction.guild_id.unwrap().0 as i64;
    let user_from_id = reaction.user_id.unwrap().0 as i64;
    let user_to_id = {
        let message = reaction.message(&ctx.http).await?;
        message.author.id.0 as i64
    };
    let channel_id = reaction.channel_id.0 as i64;
    let message_id = reaction.message_id.0 as i64;

    Ok((guild_id, user_from_id, user_to_id, channel_id, message_id))
}

async fn update_roles(
    ctx: &Context,
    database: &Database,
    member: &mut Member,
) -> Result<(), KowalskiError> {
    // Never update roles of bots
    if member.user.bot {
        return Ok(());
    }

    // Get guild and user ids
    let guild_db_id = database.get_guild(member.guild_id).await?;
    let user_db_id = database.get_user(member.guild_id, member.user.id).await?;

    // Get the score of the user
    let score = {
        let row = database
            .client
            .query_one(
                "
        SELECT SUM(CASE WHEN upvote THEN 1 ELSE -1 END) score
        FROM score_reactions r
        INNER JOIN score_emojis se ON r.guild = se.guild AND r.emoji = se.emoji
        WHERE r.guild = $1::BIGINT AND user_to = $2::BIGINT
        ",
                &[&guild_db_id, &user_db_id],
            )
            .await?;

        row.get::<_, Option<i64>>(0).unwrap_or_default()
    };

    // Get all roles handled by the level-up system
    let handled: Vec<_> = {
        let rows = database
            .client
            .query(
                "SELECT DISTINCT role FROM score_roles WHERE guild = $1::BIGINT",
                &[&guild_db_id],
            )
            .await?;

        rows.iter()
            .map(|row| RoleId(row.get::<_, i64>(0) as u64))
            .collect()
    };

    // Get all roles the user should currently have
    let current: Vec<_> = {
        let rows = database
            .client
            .query(
                "
            WITH role_score AS (
                SELECT score
                FROM score_roles
                WHERE (score >= 0 AND score <= $2::BIGINT)
                    OR (score < 0 AND score >= $2::BIGINT)
                ORDER BY ABS(score) DESC
                LIMIT 1
            )

            SELECT role
            FROM score_roles
            WHERE guild = $1::BIGINT
                AND score = (SELECT score FROM role_score)
            ",
                &[&guild_db_id, &score],
            )
            .await?;

        rows.iter()
            .map(|row| RoleId(row.get::<_, i64>(0) as u64))
            .collect()
    };

    // Current roles of the user
    let roles = &member.roles;

    // Filter roles the user should have but doesn't
    let add: Vec<_> = current
        .iter()
        .filter(|role| !roles.contains(role))
        .copied()
        .collect();
    // Filter roles the user shouldn't have but does
    let remove: Vec<_> = roles
        .iter()
        .filter(|role| handled.contains(role) && !current.contains(role))
        .copied()
        .collect();

    // Add new roles
    if !add.is_empty() {
        member.add_roles(&ctx.http, &add[..]).await?;
    }
    // Remove old roles
    if !remove.is_empty() {
        member.remove_roles(&ctx.http, &remove[..]).await?;
    }

    Ok(())
}

async fn auto_moderate(
    ctx: &Context,
    database: &Database,
    guild_id: GuildId,
    message: Message,
) -> Result<(), KowalskiError> {
    // Get guild and message ids
    let guild_db_id = database.get_guild(guild_id).await?;
    let message_db_id = database
        .get_message(guild_id, message.channel_id, message.id)
        .await?;

    // Get scores of auto-pin and auto-delete
    let pin_score = {
        let row = database
            .client
            .query_opt(
                "
        SELECT score FROM score_auto_pin
        WHERE guild = $1::BIGINT
        ",
                &[&guild_db_id],
            )
            .await?;

        row.map(|row| row.get::<_, i64>(0))
    };

    let delete_score = {
        let row = database
            .client
            .query_opt(
                "
        SELECT score FROM score_auto_delete
        WHERE guild = $1::BIGINT
        ",
                &[&guild_db_id],
            )
            .await?;

        row.map(|row| row.get::<_, i64>(0))
    };

    // Check whether auto moderation is enabled
    if pin_score.is_some() || delete_score.is_some() {
        // Get score of the message
        let score = {
            let row = database
                .client
                .query_one(
                    "
                SELECT SUM(CASE WHEN upvote THEN 1 ELSE -1 END) FROM score_reactions r
                INNER JOIN score_emojis se ON r.guild = se.guild AND r.emoji = se.emoji
                WHERE r.guild = $1::BIGINT AND message = $2::BIGINT
                ",
                    &[&guild_db_id, &message_db_id],
                )
                .await?;

            row.get::<_, Option<i64>>(0).unwrap_or_default()
        };

        // Check whether message should get pinned
        if !message.pinned {
            if let Some(pin_score) = pin_score {
                // Check whether scores share the same sign
                if (score >= 0) == (pin_score >= 0) {
                    if score.abs() >= pin_score.abs() {
                        // Pin the message
                        message.pin(&ctx.http).await?;
                    }
                }
            }
        }

        // Check whether message should get deleted
        if let Some(delete_score) = delete_score {
            // Check whether scores share the same sign
            if (score >= 0) == (delete_score >= 0) {
                if score.abs() >= delete_score.abs() {
                    // Delete the message
                    message.delete(&ctx.http).await?;
                }
            }
        }
    }

    Ok(())
}
