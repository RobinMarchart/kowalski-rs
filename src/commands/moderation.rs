use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
};
use sqlx::query;

use crate::{
    config::Command,
    data,
    database::client::Database,
    error::KowalskiError,
    error::KowalskiError::DiscordApiError,
    strings::ERR_CMD_ARGS_INVALID,
    utils::{parse_arg, send_response},
};

enum Moderation {
    Pin,
    Delete,
}

impl Display for Moderation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Moderation::Pin => "Auto-pin",
            Moderation::Delete => "Auto-delete",
        };

        write!(f, "{}", name)
    }
}

impl FromStr for Moderation {
    type Err = KowalskiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pin" => Ok(Moderation::Pin),
            "delete" => Ok(Moderation::Delete),
            _ => Err(DiscordApiError(ERR_CMD_ARGS_INVALID.to_string())),
        }
    }
}

pub async fn execute(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    command_config: &Command,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);

    let options = &command.data.options;

    // Parse first argument
    let moderation = Moderation::from_str(parse_arg(options, 0)?).unwrap();

    let guild_id = command.guild_id.unwrap();

    // Get guild id
    let guild_db_id = database.get_guild(guild_id).await?;

    let title = format!("{} message", moderation);

    if options.len() > 1 {
        // Parse second argument
        let score: i64 = parse_arg(options, 1)?;

        // Insert or update entry
        match moderation {
            Moderation::Pin => {
                query!(
                    "
                        INSERT INTO score_auto_pin(guild,score)
                        VALUES ($1, $2)
                        ON CONFLICT (guild) DO UPDATE SET score = $2
                        ",
                    guild_db_id,
                    score,
                )
                .execute(database.db())
                .await?;
            }
            Moderation::Delete => {
                query!(
                    "
                        INSERT INTO score_auto_delete(guild,score)
                        VALUES ($1, $2)
                        ON CONFLICT (guild) DO UPDATE SET score = $2
                        ",
                    guild_db_id,
                    score,
                )
                .execute(database.db())
                .await?;
            }
        }

        send_response(
            &ctx,
            &command,
            command_config,
            &title,
            &format!(
                "Moderation tool '{}' is now enabled at a score of {}.",
                moderation, score
            ),
        )
        .await
    } else {
        // Delete moderation
        match moderation {
            Moderation::Pin => {
                query!(
                        "
                        DELETE FROM score_auto_pin
                        WHERE guild = $1
                        ",
                        guild_db_id,
                    ).execute(database.db())
                    .await?;
            }
            Moderation::Delete => {
                query!(
                        "
                        DELETE FROM score_auto_delete
                        WHERE guild = $1
                        ",
                        guild_db_id,
                    ).execute(database.db())
                    .await?;
            }
        }

        send_response(
            &ctx,
            &command,
            command_config,
            &title,
            &format!("Moderation tool '{}' is now disabled.", moderation),
        )
        .await
    }
}
