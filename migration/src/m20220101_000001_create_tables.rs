use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, Statement, TransactionTrait, TransactionError},
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
                    for s in include_str!("create_tables_up.sql")
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
    /*    match manager
            .get_connection()
            .transaction(|conn| {
                Box::pin(async move {
                    for s in [
                        "guilds",
                        "users",
                        "channels",
                        "roles",
                        "messages",
                        "emojis",
                        "modules",
                        "publishing",
                        "score_auto_delete",
                        "score_auto_pin",
                        "score_cooldowns",
                        "score_drops",
                        "score_emojis",
                        "score_reactions",
                        "score_roles",
                        "reaction_roles",
                        "reminders",
                        "owned_guilds",
                    ] {
                        conn.execute(Statement::from_string(
                            manager.get_database_backend(),
                            format!("DROP TABLE {};", s),
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
        }*/
        Ok(())
    }
}
