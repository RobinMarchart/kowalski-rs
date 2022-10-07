use sea_orm_migration::prelude::*;

use crate::run_transaction;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        run_transaction(
            manager,
            include_str!("add_ids_up.sql")
                .split(";")
                .map(|s| format!("{};", s)),
        )
        .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        todo!()
    }
}
