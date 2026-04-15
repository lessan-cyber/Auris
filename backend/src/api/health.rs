use crate::AppState;
use axum::{Json, extract::State};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub s3: String,
    pub version: &'static str,
}

pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    // Check database
    let database_status = match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => "up".to_string(),
        Err(e) => {
            tracing::error!("Database health check failed: {}", e);
            "down".to_string()
        }
    };

    // Check S3
    // We try to list objects with a limit of 1 to verify connectivity and bucket existence
    let s3_status = match state
        .s3
        .client
        .list_objects_v2()
        .bucket(&state.s3.bucket_name)
        .max_keys(1)
        .send()
        .await
    {
        Ok(_) => "up".to_string(),
        Err(e) => {
            tracing::error!("S3 health check failed: {}", e);
            "down".to_string()
        }
    };

    let overall_status = if database_status == "up" && s3_status == "up" {
        "ok"
    } else {
        "unhealthy"
    };

    Json(HealthResponse {
        status: overall_status.to_string(),
        database: database_status,
        s3: s3_status,
        version: env!("CARGO_PKG_VERSION"),
    })
}
