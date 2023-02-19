use std::collections::HashMap;

use crate::{database::client::Database, error::KowalskiError};
use serenity::{
    async_trait,
    futures::TryStreamExt,
    model::prelude::{GuildId, RoleId, UserId},
    Client,
};
use sqlx::{query, query_scalar};

use super::Migration;

#[derive(Default)]
pub struct GivenRoles {}

#[async_trait]
impl Migration for GivenRoles {
    async fn migrate(self, db: &Database, client: &Client) -> Result<(), KowalskiError> {
        let db = db.begin().await?;
        //guilds we have to consider
        let guilds = query_scalar!("SELECT guild FROM guilds")
            .fetch_all(&mut db)
            .await?;

        for guild in guilds {
            let state: HashMap<RoleId, Vec<UserId>> = HashMap::new();
            GuildId(guild as u64)
                .members_iter(client.cache_and_http.http)
                .try_for_each(|user| {
                    for role in user.roles {
                        state
                            .entry(role)
                            .or_insert_with(|| Vec::new())
                            .push(user.user.id);
                    }
                    async { Ok(()) }
                })
                .await?;
            for role in query!("SELECT _reaction_roles_id AS id, _role_id as role_id, role, slots FROM reaction_roles_v WHERE guild=$1 AND slots IS NOT NULL",guild).fetch_all(&mut db).await?{
                if let Some(users)=state.get(&RoleId(role.role.unwrap() as u64)){
                    query!("UPDATE reaction_roles SET slots=$1 WHERE id = $2",role.slots.unwrap()+(users.len() as i32),role.id.unwrap()).execute(&mut db).await?;
                    for user in users{
                        query!(r#"
                            WITH "user" AS
                                (INSERT INTO users ("user",guild) VALUES ($2,$1)
                                ON CONFLICT ("user",guild) DO UPDATE SET "user" = $2
                                RETURNING id)
                            INSERT INTO given_roles ("user","role")
                            SELECT id, $3 FROM "user"
                            ON CONFLICT DO NOTHING
                            "#,guild,user.0 as i64,role.role_id.unwrap()).execute(&mut db).await?;
                    }
                }
            }
            for role in query!(
                "SELECT role, _role_id as role_id FROM score_roles_v WHERE guild = $1",
                guild
            )
            .fetch_all(&mut db)
            .await?
            {
                if let Some(users)=state.get(&RoleId(role.role.unwrap() as u64)){
                    for user in users{
                        query!(r#"
                            WITH "user" AS
                                (INSERT INTO users ("user",guild) VALUES ($2,$1)
                                ON CONFLICT ("user",guild) DO UPDATE SET "user" = $2
                                RETURNING id)
                            INSERT INTO given_roles ("user","role")
                            SELECT id, $3 FROM "user"
                            ON CONFLICT DO NOTHING
                            "#,guild,user.0 as i64,role.role_id.unwrap()
                        ).execute(&mut db).await?;
                    }
                }
            }
        }

        //try to reconstruct reaction role slots;
        db.commit().await?;
        Ok(())
    }
}
