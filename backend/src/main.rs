mod config;
mod models;
mod settings;
use std::sync::Arc;
mod api;
mod errors;
use anyhow::Result;
//use aws_sdk_s3::Config;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

pub struct AppState {
    pub db: sqlx::PgPool,
    pub s3: config::S3Client,
    pub settings: settings::Settings,
}

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
    let db_pool = config::create_pool(&settings.database_url).await?;
    // test database connection
    config::check_connection(&db_pool).await?;
    info!("Database connection is alive");

    // connect to S3
    let s3_client = config::S3Client::new(&settings).await?;
    s3_client.list_buckets().await?;
    //info!("S3 connection is alive");

    // setup app state
    let state = Arc::new(AppState {
        db: db_pool,
        s3: s3_client,
        settings: settings.clone(),
    });
    // -- router --
    let app = api::create_router(state);
    // -- Serve --
    let listener = tokio::net::TcpListener::bind(&settings.server_addr).await?;
    info!("Server listening on {}", settings.server_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
