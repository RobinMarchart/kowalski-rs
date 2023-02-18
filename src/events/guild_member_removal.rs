use std::time::Duration;

use serenity::{
    builder::CreateActionRow,
    client::Context,
    model::{
        channel,
        guild::Member,
        id::{ChannelId, GuildId},
        interactions::message_component::ButtonStyle,
        user::User,
    },
    prelude::Mentionable,
};
use sqlx::{query, query_scalar};

use crate::{
    config::Config, data, database::client::Database, error::KowalskiError, utils::create_embed,
};

pub async fn guild_member_removal(
    ctx: &Context,
    guild_id: GuildId,
    user: User,
    _member_data: Option<Member>,
) -> Result<(), KowalskiError> {
    // Get config and database
    let (config, database) = data!(ctx, (Config, Database));

    // Get guild and user ids
    let guild_db_id = guild_id.0 as i64;
    let user_db_id = user.id.0 as i64;

    // Get guild status
    let score_enabled = query_scalar!("SELECT score FROM modules WHERE guild = $1", guild_db_id)
        .fetch_optional(database.db())
        .await?
        .unwrap_or(false);

    // Check if the score module is enabled
    if score_enabled {
        // Select a random channel to send the message to
        let channel = query_scalar!(
            "
            SELECT channel FROM score_drops_v
            WHERE guild = $1
            OFFSET FLOOR(RANDOM() * (SELECT COUNT(*) FROM score_drops_v WHERE guild = $1))
            LIMIT 1
            ",
            guild_db_id
        )
        .fetch_optional(database.db())
        .await?
        .flatten()
        .map(|id| ChannelId(id as u64));

        if let Some(channel) = channel {
            // Get the score of the user
            let score = query_scalar!(
                "
                        SELECT SUM(CASE WHEN upvote THEN 1 ELSE -1 END) score
                        FROM score_reactions_v
                        WHERE guild = $1 AND user_to = $2
                        ",
                guild_db_id,
                user_db_id
            )
            .fetch_optional(database.db())
            .await?
            .flatten()
            .unwrap_or_default();

            let title = format!("User {} has dropped a score of {}", user.name, score);

            // Create action row
            let mut row = CreateActionRow::default();
            row.create_button(|button| {
                button
                    .label("Pick up the score")
                    .custom_id("pick up")
                    .style(ButtonStyle::Primary)
            });

            // Create embed
            let embed = create_embed(
                &title,
                &format!(
                    "Click the button to pick up the score of the user {}!",
                    user.mention()
                ),
            );

            // Send embed
            let mut message = channel
                .send_message(&ctx.http, |message| {
                    message
                        .set_embeds(vec![embed])
                        .components(|components| components.set_action_rows(vec![row]))
                })
                .await?;

            let interaction = message
                .await_component_interaction(&ctx.shard)
                .timeout(Duration::from_secs(config.general.pickup_timeout))
                .await;

            match interaction {
                Some(interaction) => {
                    // Get interaction user id
                    let interaction_user_db_id =
                        database.get_user(guild_id, interaction.user.id).await?;

                    // Move the reactions to the other user
                    query!("
                            UPDATE score_reactions
                            SET user_to = $3, native = false
                            FROM users u
                            WHERE u.id = user_to AND u.guild = $1 AND u.user = $2
                            ",guild_db_id,user_db_id,interaction_user_db_id).execute(database.db()).await?;

                    let embed = create_embed(
                        &title,
                        &format!(
                            "The user {} has picked up the score of {}!",
                            interaction.user.mention(),
                            user.mention()
                        ),
                    );

                    message
                        .edit(&ctx.http, |message| {
                            message
                                .components(|components| components.set_action_rows(vec![]))
                                .set_embeds(vec![embed])
                        })
                        .await?;
                }
                None => {
                    let embed =
                        create_embed(&title, "No one has picked up the reactions in time :(");

                    message
                        .edit(&ctx.http, |message| {
                            message
                                .components(|components| components.set_action_rows(vec![]))
                                .set_embeds(vec![embed])
                        })
                        .await?;
                }
            };
        }
    }

    // If no drops take place/got picked up, just delete the user
    database
        .client
        .execute(
            "
                DELETE FROM users
                WHERE guild = $1::BIGINT AND \"user\" = $2::BIGINT
                ",
            &[&guild_db_id, &user_db_id],
        )
        .await?;

    Ok(())
}
