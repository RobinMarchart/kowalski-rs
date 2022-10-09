use std::error::Error;

use tracing::error;

use kowalski_rs::{client::Client, strings::ERR_CLIENT};
use tracing_log::LogTracer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    LogTracer::init()?;

    // Create kowalski
    let mut kowalski = Client::default().await?;

    // Start kowalski
    if let Err(why) = kowalski.start().await {
        error!("{}: {}", ERR_CLIENT, why);
    }

    Ok(())
}
