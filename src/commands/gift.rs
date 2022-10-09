use std::{cmp::min, time::Duration};

use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue::User,
    },
    prelude::Mentionable,
};
use sqlx::{query_scalar, query};

use crate::{
    config::{Command, Config},
    data,
    database::client::Database,
    error::KowalskiError,
    pluralize,
    utils::{parse_arg, parse_arg_resolved, send_confirmation, send_response, InteractionResponse},
};

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), KowalskiError> {
    // Get config and database
    let (config, database) = data!(ctx, (Config, Database));

    let options = &command.data.options;

    // Parse arguments
    let user = match parse_arg_resolved(options, 0)? {
        User(user, ..) => user,
        _ => unreachable!(),
    };
    let score: i64 = parse_arg(options, 1)?;

    let guild_id = command.guild_id.unwrap();

    // Get guild and user ids
    let guild_db_id = database.get_guild(guild_id).await?;
    let user_from_db_id = database.get_user(guild_id, command.user.id).await?;
    let user_to_db_id = database.get_user(guild_id, user.id).await?;

    // Calculate amount to gift
    let amount = {
        // Select all upvotes the user has received
        let upvotes = query_scalar!(
                "
        SELECT COUNT(*) FROM score_reactions r
        INNER JOIN score_emojis se ON r.emoji = se.id
        WHERE user_to = $1::BIGINT AND upvote
        ",
                user_from_db_id,
            ).fetch_one(database.db())
            .await?.unwrap_or_default();


        min(score, upvotes)
    };

    let title = format!(
        "Gifting {} to {}",
        pluralize!("reaction", amount),
        user.name
    );

    // Prevent user from gifting to themselves
    if user.id == command.user.id {
        return send_response(
            ctx,
            command,
            command_config,
            &title,
            "You can't give reactions to yourself...",
        )
        .await;
    }

    // Check for the interaction response
    let response = send_confirmation(
        ctx,
        command,
        command_config,
        &format!(
            "Are you really sure you want to give {} reactions to {}?
                This cannot be reversed!",
            amount,
            user.mention()
        ),
        Duration::from_secs(config.general.interaction_timeout),
    )
    .await?;

    match response {
        Some(InteractionResponse::Continue) => {
            // Move reactions to the new user
            let altered_rows = query!(
                    "
                UPDATE score_reactions
                SET user_to = $2::BIGINT, native = false
                WHERE id IN (
                        SELECT r.id
                        FROM score_reactions r INNER JOIN score_emojis se ON r.emoji=se.id
                        WHERE r.user_to=$1 AND upvote
                        ORDER BY native
                        LIMIT $3
                );
                ",
                 user_from_db_id, user_to_db_id, amount,
                ).execute(database.db())
                .await?.rows_affected();

            send_response(
                ctx,
                command,
                command_config,
                &title,
                &format!(
                    "Successfully gifted {} reactions to {}.",
                    altered_rows,
                    user.mention()
                ),
            )
            .await
        }
        Some(InteractionResponse::Abort) => {
            send_response(ctx, command, command_config, &title, "Aborted the action.").await
        }
        None => Ok(()),
    }
}
