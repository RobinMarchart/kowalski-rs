pub use sea_orm_migration::prelude::*;

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

pub fn trap(){
    unsafe{std::arch::asm!("int3")}
    }
