use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, Statement, TransactionError, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager
            .get_connection()
            .transaction(|conn| {
                Box::pin(async move {
                    for s in include_str!("add_ids_up.sql")
                    .split(";")
                    .map(|s| format!("{};", s))
                    {
                        conn.execute(Statement::from_string(
                            sea_orm::DatabaseBackend::Postgres,
                            s,
                        ))
                        .await?;
                    }

                    Ok(())
                })
            })
            .await
        {
            Err(TransactionError::Connection(e)) => Err(e),
            Err(TransactionError::Transaction(e)) => Err(e),
            _ => Ok(()),
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        match manager
            .get_connection()
            .transaction(|conn| {
                Box::pin(async move {
                    for s in r#"

ALTER TABLE users DROP CONSTRAINT users_pkey;
ALTER TABLE users DROP COLUMN id;
ALTER TABLE users ADD PRIMARY KEY(guild,"user");
ALTER TABLE users DROP CONSTRAINT unique_guild_user;

ALTER TABLE channels DROP CONSTRAINT channels_pkey;
ALTER TABLE channels DROP COLUMN id;
ALTER TABLE channels ADD PRIMARY KEY(guild,channel);
ALTER TABLE channels DROP CONSTRAINT unique_guild_channel;

ALTER TABLE roles DROP CONSTRAINT roles_pkey;
ALTER TABLE roles DROP COLUMN id;
ALTER TABLE roles ADD PRIMARY KEY(guild,role);
ALTER TABLE roles DROP CONSTRAINT unique_guild_role;

"#
                    .split(";")
                    .map(|s| format!("{};", s))
                    {
                        conn.execute(Statement::from_string(
                            sea_orm::DatabaseBackend::Postgres,
                            s,
                        ))
                        .await?;
                    }

                    Ok(())
                })
            })
            .await
        {
            Err(TransactionError::Connection(e)) => Err(e),
            Err(TransactionError::Transaction(e)) => Err(e),
            _ => Ok(()),
        }
    }
}
