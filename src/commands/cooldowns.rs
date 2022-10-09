use itertools::Itertools;
use serenity::{
    client::Context,
    model::{id::RoleId, interactions::application_command::ApplicationCommandInteraction},
    prelude::Mentionable,
};
use sqlx::query;

use crate::{
    config::Command, config::Config, data, database::client::Database, error::KowalskiError,
    utils::send_response,
};

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), KowalskiError> {
    // Get config and database
    let (config, database) = data!(ctx, (Config, Database));

    let guild_id = command.guild_id.unwrap();

    // Get guild id
    let guild_db_id = database.get_guild(guild_id).await?;

    // Get roles and their respective cooldowns
    let role_cooldowns: Vec<_> = {
        let rows =
            query!(
                "
                SELECT roles.role, score_cooldowns.cooldown FROM score_cooldowns INNER JOIN roles ON score_cooldowns.role=roles.id
                WHERE roles.guild = $1
                ORDER BY cooldown
                ",
                guild_db_id,
            ).fetch_all(database.db())
            .await?;

        rows.iter()
            .map(|row| (RoleId(row.role as u64), row.cooldown))
            .collect()
    };

    let role_cooldowns = role_cooldowns
        .iter()
        .map(|&(role_id, cooldown)| format!("{}: {} seconds", role_id.mention(), cooldown))
        .join("\n");

    // Get default cooldown
    let default_cooldown = config.general.default_cooldown;

    if role_cooldowns.is_empty() {
        send_response(
            &ctx,
            &command,
            &command_config,
            "Cooldowns",
            &format!(
                "Everyone has a reaction cooldown of {} seconds.",
                default_cooldown
            ),
        )
        .await
    } else {
        send_response(
            &ctx,
            &command,
            &command_config,
            "Cooldowns",
            &format!(
                "The default reaction cooldown is set to {} seconds.

                The following roles have custom cooldowns defined (smallest applies):
                {}",
                default_cooldown, role_cooldowns
            ),
        )
        .await
    }
}
