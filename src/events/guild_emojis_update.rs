use std::collections::HashMap;

use sea_orm::{EntityTrait, QueryFilter};
use serenity::{
    client::Context,
    model::{
        guild::Emoji,
        id::{EmojiId, GuildId},
    },
};

use crate::{data, database::{client::Database, entity::emojis}, error::KowalskiError};

pub async fn guild_emojis_update(
    ctx: &Context,
    guild_id: GuildId,
    current_state: HashMap<EmojiId, Emoji>,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);

    // Get guild id
    let guild_db_id = guild_id.0 as i64;

    // Get all emojis tracked by the database for this guild
    let emoji_list=emojis::Entity::find().filter(emojis::Column::Guild.eq(guild_db_id)).all(&database).await?;

    for emoji in emoji_list {
        // Check whether emoji still exists
        if !current_state.contains_key(&emoji.guild_emoji.unwrap()) {

            // Delete the emoji
            emojis::Entity::delete_by_id(emoji.id).exec(&database).await?;


        }
    }

    Ok(())
}
