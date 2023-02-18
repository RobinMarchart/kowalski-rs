use dotenvy::var;

use sqlx::{migrate, PgPool};
use tracing::info;

fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    //LogTracer::init()?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let db = PgPool::connect(
                &var("DB_CONF").expect("Missing environment variable: DB_CONF"),
            )
            .await?;
            info!("Connected to database");
            info!("applieing migrations");
            //apply migrations
            migrate!().run(&db).await?;
            info!("Database setup successful");
            db.close().await;
            Ok(())
        })
}
