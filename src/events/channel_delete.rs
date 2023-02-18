use serenity::{client::Context, model::channel::GuildChannel};
use sqlx::query;

use crate::data;
use crate::database::client::Database;
use crate::error::KowalskiError;

pub async fn channel_delete(ctx: &Context, channel: &GuildChannel) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx,Database);

    // Get guild and channel ids
    let guild_id = channel.guild_id.0 as i64;
    let channel_id = channel.id.0 as i64;

    query!("DELETE FROM channels WHERE guild = $1 AND channel = $2",guild_id,channel_id)
        .execute(database.db()).await?;

    Ok(())
}
