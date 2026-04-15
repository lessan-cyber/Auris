use crate::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use std::sync::Arc;
use tokio::time::{Duration, timeout};
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub s3: String,
    pub version: &'static str,
}

pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<HealthResponse>) {
    // Check database
    let database_status = match timeout(
        Duration::from_secs(2),
        sqlx::query("SELECT 1").execute(&state.db),
    )
    .await
    {
        Ok(Ok(_)) => "up".to_string(),
        Ok(Err(e)) => {
            tracing::error!("Database health check failed: {}", e);
            "down".to_string()
        }
        Err(_) => {
            tracing::error!("Database health check timed out");
            "down".to_string()
        }
    };

    // Check S3
    // We try to list objects with a limit of 1 to verify connectivity and bucket existence

    let s3_status = match timeout(
        Duration::from_secs(2),
        state
            .s3
            .client
            .list_objects_v2()
            .bucket(&state.s3.bucket_name)
            .max_keys(1)
            .send(),
    )
    .await
    {
        Ok(Ok(_)) => "up".to_string(),
        Ok(Err(e)) => {
            tracing::error!("S3 health check failed: {}", e);
            "down".to_string()
        }
        Err(_) => {
            tracing::error!("S3 health check timed out");
            "down".to_string()
        }
    };

    let overall_status = if database_status == "up" && s3_status == "up" {
        "ok"
    } else {
        "unhealthy"
    };
    let body = HealthResponse {
        status: overall_status.to_string(),
        database: database_status,
        s3: s3_status,
        version: env!("CARGO_PKG_VERSION"),
    };
    let code = if overall_status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body))
}
