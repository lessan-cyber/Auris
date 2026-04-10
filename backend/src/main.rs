mod config;
mod models;
mod settings;
use std::sync::Arc;
mod api;
mod errors;
mod fingerprint;
mod worker;
mod utils;
use anyhow::Result;
use worker::mode::{ExecutionMode, Mode};
//use aws_sdk_s3::Config;
use clap::Parser;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

pub struct AppState {
    pub db: sqlx::PgPool,
    pub s3: config::S3Client,
    pub settings: settings::Settings,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mode = Mode::parse();
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

    match mode.execution_mode {
        ExecutionMode::Worker => {
            // Run only the worker
            worker::run_worker(state).await;
        }
        ExecutionMode::Api => {
            // Run API server (existing code)
            let app = api::create_router(state);
            let listener = tokio::net::TcpListener::bind(&settings.server_addr).await?;
            axum::serve(listener, app).await?;
        }
    }
    Ok(())
}
