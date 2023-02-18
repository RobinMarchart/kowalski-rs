#[cfg(feature = "event-calendar")]
pub mod calendar;
pub mod client;
pub mod commands;
pub mod config;
pub mod cooldowns;
pub mod credits;
pub mod database;
pub mod error;
pub mod events;
pub mod history;
#[cfg(feature = "nlp-model")]
pub mod model;
pub mod reminders;
pub mod strings;
pub mod utils;


use std::error::Error;

use tracing::error;

use crate::{client::Client, strings::ERR_CLIENT};
use tracing_log::LogTracer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    LogTracer::init()?;

    dotenvy::dotenv()?;
    // Create kowalski
    let mut kowalski = Client::default().await?;

    // Start kowalski
    if let Err(why) = kowalski.start().await {
        error!("{}: {}", ERR_CLIENT, why);
    }

    Ok(())
}
