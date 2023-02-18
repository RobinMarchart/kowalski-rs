use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    time::Duration,
};

use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
};
use sqlx::{query, query_as};
use tokio::sync::Mutex;

use crate::{
    config::{Command, Config, Module},
    data,
    database::{client::Database, types::Modules},
    error::KowalskiError,
    error::KowalskiError::DiscordApiError,
    strings::ERR_CMD_ARGS_INVALID,
    utils::{
        create_module_command, parse_arg, send_confirmation, send_failure, send_response,
        InteractionResponse,
    },
};

/// Lock to avoid race conditions when modifying the status object
static LOCK: Mutex<i32> = Mutex::const_new(1);

enum Action {
    Enable,
    Disable(bool),
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Action::Enable => "Enable",
            Action::Disable(remove) => {
                if *remove {
                    "Remove"
                } else {
                    "Disable"
                }
            }
        };

        write!(f, "{}", name)
    }
}

impl FromStr for Action {
    type Err = KowalskiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enable" => Ok(Action::Enable),
            "disable" => Ok(Action::Disable(false)),
            "remove" => Ok(Action::Disable(true)),
            _ => Err(DiscordApiError(ERR_CMD_ARGS_INVALID.to_string())),
        }
    }
}

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), KowalskiError> {
    // Get config and database
    let (config, database) = data!(ctx, (Config, Database));

    let options = &command.data.options;

    // Parse arguments
    let action = Action::from_str(parse_arg(options, 0)?).unwrap();
    let module = Module::from_str(parse_arg(options, 1)?).unwrap();

    // Disable the module in private channels
    if matches!(command.guild_id, None) {
        send_failure(
            &ctx,
            command,
            "Command not available",
            "Module commands are only available on guilds.",
        )
        .await;

        return Ok(());
    }

    // Only allow the owner to enable/disable the owner module
    if matches!(module, Module::Owner) {
        if !config.general.owners.contains(&command.user.id.0) {
            send_failure(
                &ctx,
                command,
                "Insufficient permissions",
                "I'm sorry, but this module is restricted.",
            )
            .await;

            return Ok(());
        }
    }

    let guild_id = command.guild_id.unwrap();

    // Get guild id
    let guild_db_id = database.get_guild(guild_id).await?;

    query!(
        "INSERT INTO modules(guild) VALUES ($1) ON CONFLICT DO NOTHING",
        guild_db_id
    )
    .execute(database.db())
    .await?;

    // Update the status object
    let enable = matches!(action, Action::Enable);
    let changed = 1
        == match module {
            Module::Owner => query!(
                "UPDATE modules SET owner = $1 WHERE guild = $2 AND NOT owner = $1;",
                enable,
                guild_db_id
            )
            .execute(database.db())
            .await?
            .rows_affected(),
            Module::Utility => query!(
                "UPDATE modules SET utility = $1 WHERE guild = $2 AND NOT utility = $1;",
                enable,
                guild_db_id
            )
            .execute(database.db())
            .await?
            .rows_affected(),
            Module::Score => query!(
                "UPDATE modules SET score = $1 WHERE guild = $2 AND NOT score = $1;",
                enable,
                guild_db_id
            )
            .execute(database.db())
            .await?
            .rows_affected(),

            Module::ReactionRoles => query!(
            "UPDATE modules SET reaction_roles = $1 WHERE guild = $2 AND NOT reaction_roles = $1;",
            enable,
            guild_db_id
        )
            .execute(database.db())
            .await?
            .rows_affected(),

            Module::Analyze => query!(
                r#"UPDATE modules SET "analyze" = $1 WHERE guild = $2 AND NOT "analyze" = $1;"#,
                enable,
                guild_db_id
            )
            .execute(database.db())
            .await?
            .rows_affected(),
        };

    // Get title of the embed
    let title = format!("{} module '{:?}'", action, module);

    match action {
        Action::Disable(true) => {
            // Check for the interaction response
            let response = send_confirmation(
                ctx,
                command,
                command_config,
                &format!("Are you really sure you want to remove all of the module data provided by the module '{:?}'?
                This cannot be reversed, all data will be gone permanently!", module),
                Duration::from_secs(config.general.interaction_timeout),
            )
            .await?;

            match response {
                Some(InteractionResponse::Continue) => {
                    remove(ctx, command, command_config, title, module).await
                }
                Some(InteractionResponse::Abort) => {
                    send_response(ctx, command, command_config, &title, "Aborted the action.").await
                }
                None => Ok(()),
            }
        }
        _ => {
            // Enable/disable the module
            match status {
                Some(status) => {
                    send_response(
                        ctx,
                        command,
                        command_config,
                        &title,
                        "I'm updating the module... This can take some time.",
                    )
                    .await?;

                    // Update the guild commands
                    create_module_command(ctx, &config, guild_id, &status).await;

                    send_response(
                        ctx,
                        command,
                        command_config,
                        &title,
                        "I have updated the module.",
                    )
                    .await
                }
                None => {
                    // No real update
                    send_response(
                        ctx,
                        command,
                        command_config,
                        &title,
                        "The state of the module did not change. No need to update anything.",
                    )
                    .await
                }
            }
        }
    }
    if changed {
        send_response(
            ctx,
            command,
            command_config,
            &title,
            "I'm updating the module... This can take some time.",
        )
        .await?;

        // Update the guild commands

        let modules = query_as!(
            Modules,
            r#"
SELECT owner,utility,score,reaction_roles,"analyze"
FROM modules WHERE guild=$1;"#,
            guild_db_id
        )
        .fetch_one(database.db())
        .await?;

        create_module_command(ctx, &config, guild_id, &modules).await;

        send_response(
            ctx,
            command,
            command_config,
            &title,
            "I have updated the module.",
        )
        .await
    }
}

async fn remove(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
    title: String,
    module: Module,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);
    let transaction = database.begin().await?;

    let guild_id = command.guild_id.unwrap();

    // Get guild id
    let guild_db_id = guild_id.0 as i64;

    match module {
        Module::Utility => {
            query!("DELETE FROM publishing WHERE guild = $1;", guild_db_id)
                .execute(database.db())
                .await?;

            query!(
                r#"DELETE FROM reminders WHERE id=
(SELECT r.id FROM reminders r INNER JOIN users u ON r."user"=u.id WHERE u.guild=$1);"#,
                guild_db_id
            )
            .execute(database.db())
            .await?;
        }
        Module::Score => {
            query!(
                "DELETE FROM score_auto_delete WHERE guild = $1;",
                guild_db_id
            )
            .execute(database.db())
            .await?;

            query!(
                "DELETE FROM score_auto_pin WHERE guild = $1::BIGINT;",
                guild_db_id
            )
            .execute(database.db())
            .await?;

            query!(
                "DELETE FROM score_cooldowns WHERE role IN (SELECT id FROM roles WHERE guild=$1);",
                guild_db_id
            )
            .execute(database.db())
            .await?;

            query!(
                "DELETE FROM score_drops WHERE channel IN (SELECT id FROM channels WHERE guild=$1);",
                guild_db_id
            )
            .execute(database.db())
            .await?;

            query!(
                "DELETE FROM score_emojis WHERE guild = $1::BIGINT;",
                guild_db_id
            )
            .execute(database.db())
            .await?;

            query!(
                "DELETE FROM score_roles WHERE role IN (SELECT id FROM roles WHERE guild=$1);",
                guild_db_id
            )
            .execute(database.db())
            .await?;
        }
        Module::ReactionRoles => {
            query!(
                r#"DELETE FROM reminders WHERE "user" IN (SELECT id FROM users WHERE guild=$1);"#,
                guild_db_id
            )
            .execute(database.db())
            .await?;
        }
        _ => {
            return send_response(
                ctx,
                command,
                command_config,
                &title,
                "I have updated the module. There was no need to remove any data.",
            )
            .await;
        }
    }

    transaction.commit().await?;

    send_response(
        ctx,
        command,
        command_config,
        &title,
        "I have removed all of the module data.",
    )
    .await
}
