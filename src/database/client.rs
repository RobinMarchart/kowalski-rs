use std::{env, ops::Deref};

use serenity::{
    model::{
        channel::ReactionType,
        id::{ChannelId, GuildId, MessageId, RoleId, UserId},
    },
    prelude::TypeMapKey,
};
use sqlx::{migrate, query, query_scalar, PgPool};
use tracing::info;

use crate::{
    error::KowalskiError,
    strings::{ERR_ENV_NOT_SET, INFO_DB_CONNECTED, INFO_DB_SETUP},
};

#[derive(Debug,Clone)]
/// The database client.
pub struct Database {
    db: PgPool,
}

impl Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl Database {
    pub async fn new() -> Result<Self, KowalskiError> {
        // Get database config (https://docs.rs/tokio-postgres/0.7.2/tokio_postgres/config/struct.Config.html)
        //
        let db = PgPool::connect(
            &env::var("DB_CONF").expect(&format!("{}: {}", ERR_ENV_NOT_SET, "DB_CONF")),
        )
        .await?;
        info!("{}", INFO_DB_CONNECTED);

        //apply migrations
        migrate!().run(&db).await?;
        info!("{}", INFO_DB_SETUP);

        Ok(Database { db })
    }

    /// Gets the id of a guild given the GuildId object.
    ///
    /// Note: If the guild is not registered before, it will create a new row
    pub async fn get_guild(&self, guild_id: GuildId) -> Result<i64, sqlx::Error> {
        let id = guild_id.0 as i64;

        query!(
            "
                INSERT INTO guilds VALUES ($1) ON CONFLICT DO NOTHING;
                ",
            id
        )
        .execute(self.db())
        .await?;
        Ok(id)
    }

    /// Gets the id of a user given the GuildId and UserId object.
    ///
    /// Note: If the guild or user is not registered before, it will create new rows
    pub async fn get_user(&self, guild_id: GuildId, user_id: UserId) -> Result<i64, sqlx::Error> {
        let guild_db_id = self.get_guild(guild_id).await?;
        let user_db_id = user_id.0 as i64;
        match query_scalar!(
            r#"SELECT id FROM users WHERE guild=$1 AND "user"=$2;"#,
            guild_db_id,
            user_db_id
        )
        .fetch_optional(self.db())
        .await?
        {
            Some(id) => Ok(id),
            None => {
                match query_scalar!(
                    r#"INSERT INTO users (id,guild,"user") VALUES (DEFAULT,$1,$2) RETURNING id;"#,
                    guild_db_id,
                    user_db_id
                )
                .fetch_optional(self.db())
                .await?
                {
                    Some(id) => Ok(id),
                    None => {
                        tracing::debug!(
                            "Insert race detected for guild:{}, user:{}",
                            guild_db_id,
                            user_db_id
                        );
                        match query_scalar!(
                            r#"SELECT id FROM users WHERE guild=$1 AND "user"=$2;"#,
                            guild_db_id,
                            user_db_id
                        )
                        .fetch_one(self.db())
                        .await
                        {
                            Ok(id) => Ok(id),
                            Err(e) => {
                                tracing::error!(
                                    "unable to insert guild:{},user:{}",
                                    guild_db_id,
                                    user_db_id
                                );
                                Err(e)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Gets the id of a role given the GuildId and RoleId object.
    ///
    /// Note: If the guild or role is not registered before, it will create new rows
    pub async fn get_role(&self, guild_id: GuildId, role_id: RoleId) -> Result<i64, sqlx::Error> {
        let guild_db_id = self.get_guild(guild_id).await?;
        let role_db_id = role_id.0 as i64;
        match query_scalar!(
            r#"SELECT id FROM roles WHERE guild=$1 AND role=$2;"#,
            guild_db_id,
            role_db_id
        )
        .fetch_optional(self.db())
        .await?
        {
            Some(id) => Ok(id),
            None => {
                match query_scalar!(
                    r#"INSERT INTO roles (id,guild,role) VALUES (DEFAULT,$1,$2) RETURNING id;"#,
                    guild_db_id,
                    role_db_id
                )
                .fetch_optional(self.db())
                .await?
                {
                    Some(id) => Ok(id),
                    None => {
                        tracing::debug!(
                            "Insert race detected for guild:{}, role:{}",
                            guild_db_id,
                            role_db_id
                        );
                        match query_scalar!(
                            r#"SELECT id FROM roles WHERE guild=$1 AND role=$2;"#,
                            guild_db_id,
                            role_db_id
                        )
                        .fetch_one(self.db())
                        .await
                        {
                            Ok(id) => Ok(id),
                            Err(e) => {
                                tracing::error!(
                                    "unable to insert guild:{},role:{}",
                                    guild_db_id,
                                    role_db_id
                                );
                                Err(e)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Gets the id of a channel given the GuildId and ChannelId object.
    ///
    /// Note: If the guild or role is not registered before, it will create new rows
    pub async fn get_channel(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Result<i64, sqlx::Error> {
        let guild_db_id = self.get_guild(guild_id).await?;
        let channel_db_id = channel_id.0 as i64;
        match query_scalar!(
            r#"SELECT id FROM channels WHERE guild=$1 AND channel=$2;"#,
            guild_db_id,
            channel_db_id
        )
        .fetch_optional(self.db())
        .await?
        {
            Some(id) => Ok(id),
            None => {
                match query_scalar!(
                    r#"INSERT INTO roles (id,guild,role) VALUES (DEFAULT,$1,$2) RETURNING id;"#,
                    guild_db_id,
                    channel_db_id
                )
                .fetch_optional(self.db())
                .await?
                {
                    Some(id) => Ok(id),
                    None => {
                        tracing::debug!(
                            "Insert race detected for guild:{}, channel:{}",
                            guild_db_id,
                            channel_db_id
                        );
                        match query_scalar!(
                            r#"SELECT id FROM channels WHERE guild=$1 AND channel=$2;"#,
                            guild_db_id,
                            channel_db_id
                        )
                        .fetch_one(self.db())
                        .await
                        {
                            Ok(id) => Ok(id),
                            Err(e) => {
                                tracing::error!(
                                    "unable to insert guild:{},channel:{}",
                                    guild_db_id,
                                    channel_db_id
                                );
                                Err(e)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Gets the id of a message given the GuildId and MessageId object.
    ///
    /// Note: If the guild or role is not registered before, it will create new rows
    pub async fn get_message(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<i64, sqlx::Error> {
        let channel_db_id = self.get_channel(guild_id, channel_id).await?;
        let message_db_id = message_id.0 as i64;
        match query_scalar!(
            r#"SELECT id FROM messages WHERE channel=$1 AND message=$2;"#,
            channel_db_id,
            message_db_id
        )
        .fetch_optional(self.db())
        .await?
        {
            Some(id) => Ok(id),
            None => {
                match query_scalar!(
                    r#"INSERT INTO messages (id,channel,message) VALUES (DEFAULT,$1,$2) RETURNING id;"#,
                    channel_db_id,
                    message_db_id
                )
                .fetch_optional(self.db())
                .await?
                {
                    Some(id) => Ok(id),
                    None => {
                        tracing::debug!(
                            "Insert race detected for channel:{}, message:{}",
                            channel_db_id,
                            message_db_id
                        );
                        match query_scalar!(
                            r#"SELECT id FROM messages WHERE channel=$1 AND message=$2;"#,
                            channel_db_id,
                            message_db_id
                        )
                        .fetch_one(self.db())
                        .await
                        {
                            Ok(id) => Ok(id),
                            Err(e) => {
                                tracing::error!(
                                    "unable to insert channel:{},message:{}",
                                    channel_db_id,
                                    message_db_id
                                );
                                Err(e)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Gets the id of an emoji given the reaction type.
    ///
    /// Note: If the emoji is not registered before, it will create a new row
    pub async fn get_emoji(
        &self,
        guild_id: GuildId,
        emoji: &ReactionType,
    ) -> Result<i64, sqlx::Error> {
        match emoji {
            ReactionType::Custom { id: emoji_id, .. } => {
                let guild_db_id = self.get_guild(guild_id).await?;
                let emoji_db_id = emoji_id.0 as i64;
                match query_scalar!(
            r#"SELECT id FROM emojis WHERE guild=$1 AND guild_emoji=$2;"#,
            guild_db_id,
            emoji_db_id
        )
        .fetch_optional(self.db())
        .await?
        {
            Some(id) => Ok(id),
            None => {
                match query_scalar!(
                    r#"INSERT INTO emojis (id,guild,guild_emoji) VALUES (DEFAULT,$1,$2) RETURNING id;"#,
                    guild_db_id,
                    emoji_db_id
                )
                .fetch_optional(self.db())
                .await?
                {
                    Some(id) => Ok(id),
                    None => {
                        tracing::debug!(
                            "Insert race detected for guild:{}, guild_emoji:{}",
                            guild_db_id,
                            emoji_db_id
                        );
                        match query_scalar!(
                            r#"SELECT id FROM emojis WHERE guild=$1 AND guild_emoji=$2;"#,
                            guild_db_id,
                            emoji_db_id
                        )
                        .fetch_one(self.db())
                        .await
                        {
                            Ok(id) => Ok(id),
                            Err(e) => {
                                tracing::error!(
                                    "unable to insert guild:{}, guild_emoji:{}",
                            guild_db_id,
                            emoji_db_id
                                );
                                Err(e)
                            }
                        }
                    }
                }
            }

        }
            }
            ReactionType::Unicode(string) => {
                match query_scalar!(r#"SELECT id FROM emojis WHERE unicode=$1;"#, string,)
                    .fetch_optional(self.db())
                    .await?
                {
                    Some(id) => Ok(id),
                    None => {
                        match query_scalar!(
                            r#"INSERT INTO emojis (id,unicode) VALUES (DEFAULT,$1) RETURNING id;"#,
                            string
                        )
                        .fetch_optional(self.db())
                        .await?
                        {
                            Some(id) => Ok(id),
                            None => {
                                tracing::debug!("Insert race detected for unicode:{}", string);
                                match query_scalar!(
                                    r#"SELECT id FROM emojis WHERE unicode=$1;"#,
                                    string
                                )
                                .fetch_one(self.db())
                                .await
                                {
                                    Ok(id) => Ok(id),
                                    Err(e) => {
                                        tracing::error!("unable to insert unicode:{}", string);
                                        Err(e)
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn db(&self) -> &PgPool {
        &self.db
    }
}

impl TypeMapKey for Database {
    type Value = std::sync::Arc<Database>;
}
