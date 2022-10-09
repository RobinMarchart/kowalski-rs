use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::EntityTrait;
use serenity::{client::Context, model::channel::GuildChannel};

use crate::data;
use crate::database::client::Database;
use crate::database::entity::channels;
use crate::error::KowalskiError;

pub async fn channel_delete(ctx: &Context, channel: &GuildChannel) -> Result<(), KowalskiError> {
    // Get database
    let database = ctx.data.read().await.get();

    // Get guild and channel ids
    let guild_db_id = channel.guild_id.0 as i64;
    let channel_db_id = channel.id.0 as i64;
    channels::Entity::delete(channels::ActiveModel {
        guild: Set(guild_db_id),
        channel: Set(channel_db_id),
        id: NotSet,
    })
    .exec(database)
    .await?;
    Ok(())
}
