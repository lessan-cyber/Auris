use anyhow::{Context, Result};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
pub struct Settings {
    /// Database connection string
    pub database_url: String,
    /// Server address (default: 0.0.0.0:8000)
    pub server_addr: SocketAddr,
    /// Rustfs/S3 configuration
    pub s3_endpoint: String,
    pub s3_bucket_name: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,

    /// Upload limits
    pub max_file_size: usize,
    
    /// CORS configuration
    pub cors_allowed_origins: Vec<String>,
    pub cors_allow_credentials: bool,
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let bind_addr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8000".to_string())
            .parse()
            .context("Invalid bind addr")?;
        let max_file_size = std::env::var("MAX_FILE_SIZE")
            .unwrap_or_else(|_| "52428800".to_string()) // 50 MB as default
            .parse()
            .context("Invalid max file size")?;
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("Missing DATABASE_URL")?,
            server_addr: bind_addr,
            s3_endpoint: std::env::var("RUSTFS_ENDPOINT").context("Missing RUSTFS_ENDPOINT")?,
            s3_bucket_name: std::env::var("RUSTFS_BUCKET_NAME")
                .context("Missing RUSTFS_BUCKET_NAME")?,
            s3_access_key: std::env::var("RUSTFS_ACCESS_KEY")
                .context("Missing RUSTFS_ACCESS_KEY")?,
            s3_secret_key: std::env::var("RUSTFS_SECRET_KEY")
                .context("Missing RUSTFS_SECRET_KEY")?,
            max_file_size,
            cors_allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173,http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            cors_allow_credentials: std::env::var("CORS_ALLOW_CREDENTIALS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        })
    }
}
