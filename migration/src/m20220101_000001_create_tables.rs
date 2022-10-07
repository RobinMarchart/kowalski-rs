use sea_orm_migration::prelude::*;

use crate::run_transaction;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        run_transaction(
            manager,
            include_str!("create_tables_up.sql")
                .split(";")
                .map(|s| format!("{};", s)),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        run_transaction(
            manager,
            [
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
            ]
            .into_iter()
            .map(|s| format!("DROP TABLE {};", s)),
        )
        .await
    }
}
