mod config;
mod models;
mod settings;
use anyhow::Result;
//use aws_sdk_s3::Config;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load settings
    let settings = settings::Settings::from_env()?;

    // connect to database
    let pool = config::create_pool(&settings.database_url).await?;

    // test connection
    config::check_connection(&pool).await?;
    Ok(())
}
