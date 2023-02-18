use itertools::Itertools;
use serenity::{
    client::Context,
    futures::TryStreamExt,
    model::{id::RoleId, interactions::application_command::ApplicationCommandInteraction},
    prelude::Mentionable,
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
    let guild_db_id = guild_id.0 as i64;

    // Get roles and their respective cooldowns
    let role_cooldowns: Vec<_> = query!(
        "
                SELECT r.role, score FROM score_roles sc
                INNER JOIN roles r ON r.id = sc.role
                WHERE r.guild = $1
                ORDER BY score
                ",
        guild_db_id,
    )
    .fetch(database.db())
    .map_ok(|row| (RoleId::from(row.role as u64), row.score))
    .try_collect()
    .await?;

    let levelup_roles = role_cooldowns
        .iter()
        .map(|&(role_id, cooldown)| {
            format!(
                "{}: **score {} {}**",
                role_id.mention(),
                if cooldown >= 0 { ">=" } else { "<=" },
                cooldown
            )
        })
        .join("\n");

    let title = "Level-up roles";

    if levelup_roles.is_empty() {
        send_response(
            &ctx,
            &command,
            &command_config,
            &title,
            "There are currently no level-up roles defined for this server.",
        )
        .await
    } else {
        send_response(
            &ctx,
            &command,
            &command_config,
            &title,
            &format!(
                "The following roles will get assigned to users when they reach a certain score:
                {}",
                levelup_roles
            ),
        )
        .await
    }
}
