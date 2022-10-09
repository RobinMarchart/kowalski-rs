use sea_orm::EntityTrait;
use serenity::{
    client::Context,
    model::guild::{Guild, UnavailableGuild},
};

use crate::{data, database::client::Database,database::entity::prelude::Guilds, error::KowalskiError};

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
        let guild_db_id = incomplete.id.0 as i64;

        Guilds::delete_by_id(guild_db_id).exec(&database).await?;

    }

    Ok(())
}
