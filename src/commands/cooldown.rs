use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
    model::interactions::application_command::ApplicationCommandInteractionDataOptionValue::Role,
    prelude::Mentionable,
};
use sqlx::query;

use crate::{
    config::Command,
    data,
    database::client::Database,
    error::KowalskiError,
    utils::{parse_arg, parse_arg_resolved, send_response},
};

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);

    let options = &command.data.options;

    // Parse first argument
    let role = match parse_arg_resolved(options, 0)? {
        Role(role) => role,
        _ => unreachable!(),
    };

    // Get guild and role ids
    let role_db_id = database.get_role(role.guild_id, role.id).await?;

    let title = format!("Set cooldown for {}", role.name);

    if options.len() > 1 {
        // Parse second argument
        let cooldown: i64 = parse_arg(options, 1)?;

        // Insert or update entry
        query!(
            "
        INSERT INTO score_cooldowns(role,cooldown)
        VALUES ($1, $2)
        ON CONFLICT (role)
        DO UPDATE SET cooldown = $2
        ",
            role_db_id,
            cooldown,
        )
        .execute(database.db())
        .await?;

        send_response(
            &ctx,
            &command,
            command_config,
            &title,
            &format!(
                "The role {} now has a reaction-cooldown of {} seconds.",
                role.mention(),
                cooldown
            ),
        )
        .await
    } else {
        // Delete cooldown
        query!(
            "
        DELETE FROM score_cooldowns
        WHERE role = $1
        ",
            &role_db_id,
        )
        .execute(database.db())
        .await?;

        send_response(
            &ctx,
            &command,
            command_config,
            &title,
            &format!(
                "The role {} now has the default reaction-cooldown.",
                role.mention()
            ),
        )
        .await
    }
}
