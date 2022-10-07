pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{TransactionError, Statement, TransactionTrait, ConnectionTrait};

mod m20220101_000001_create_tables;
mod m20221005_211446_add_ids;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_tables::Migration),
            Box::new(m20221005_211446_add_ids::Migration),
        ]
    }
}

pub async fn run_transaction<I: Send+Iterator<Item = String>+'static>(manager: &SchemaManager<'_>, i: I) -> Result<(), DbErr> {
    match manager
        .get_connection()
        .transaction(|conn| {
            Box::pin(async move {
                for s in i {
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

pub fn trap() {
    unsafe { std::arch::asm!("int3") }
}
