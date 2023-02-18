use serenity::{
    client::Context,
    model::guild::{Guild, UnavailableGuild},
};
use sqlx::query;

use crate::{data, database::client::Database, error::KowalskiError};

pub async fn guild_delete(
    ctx: &Context,
    incomplete: UnavailableGuild,
    _full: Option<Guild>,
) -> Result<(), KowalskiError> {
    // Check whether the bot was actually removed
    if !incomplete.unavailable {
        // Get database
        let database = data!(ctx, Database);

        // Get guild id
        let guild_id = incomplete.id.0 as i64;

        query!("DELETE FROM guilds WHERE guild = $1",guild_id)
            .execute(database.db()).await?;
    }

    Ok(())
}
