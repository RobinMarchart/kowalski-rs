use std::collections::HashMap;

use serenity::{
    client::Context,
    model::{
        guild::Emoji,
        id::{EmojiId, GuildId},
    },
};
use sqlx::query;

use crate::{data, database::client::Database, error::KowalskiError};

pub async fn guild_emojis_update(
    ctx: &Context,
    guild_id: GuildId,
    current_state: HashMap<EmojiId, Emoji>,
) -> Result<(), KowalskiError> {
    // Get database
    let database = data!(ctx, Database);

    // Get guild id
    let guild_id = guild_id.0 as i64;
    let emoji_names:Vec<i64>=current_state.keys().map(|id|id.0 as i64).collect();

    query!("DELETE FROM emojis WHERE guild = $1 AND NOT guild_emoji = ANY($2)"
    ,guild_id,&emoji_names).execute(database.db()).await?;

    Ok(())
}
