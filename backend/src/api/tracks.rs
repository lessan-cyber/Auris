use crate::AppState;
use crate::errors::{AppError, Result};
use crate::models::jobs::JobStatus;
use crate::models::tracks::{Track, TrackResponse, TrackStatus};
use crate::utils::file_validation::validate_audio_file;
use axum::extract::Path;
use axum::{
    Json, Router,
    extract::{Multipart, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::Deserialize;

use serde_json;

use std::sync::Arc;
use uuid::Uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_track))
        .route("/", get(list_tracks))
        .route("/{id}", get(get_track))
        .route("/{id}", delete(delete_track))
        .route("/{id}/url", get(get_track_url))
}

async fn create_track(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<TrackResponse>)> {
    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;

    let mut file_data: Option<bytes::Bytes> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;

    tracing::info!("Starting multipart form parsing");
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to get next multipart field: {}", e);
        AppError::BadRequest(format!("Failed to parse multipart header: {}", e))
    })? {
        let name = field.name().map(String::from);
        tracing::debug!("Processing field: {:?}", name);

        match name.as_deref() {
            Some("title") => {
                title = Some(field.text().await.map_err(|e| {
                    tracing::error!("Failed to read title text: {}", e);
                    AppError::BadRequest(format!("Invalid title: {}", e))
                })?);
            }
            Some("artist") => {
                artist = field.text().await.ok();
            }

            Some("file") => {
                file_name = field.file_name().map(String::from);
                content_type = field.content_type().map(String::from);
                tracing::info!(
                    "File field found: name={:?}, content_type={:?}",
                    file_name,
                    content_type
                );

                // Collect bytes (for production, stream to S3 instead)
                let bytes = field.bytes().await.map_err(|e| {
                    tracing::error!("Failed to read file bytes for {:?}: {}", file_name, e);
                    AppError::BadRequest(format!("Failed to read file: {}", e))
                })?;

                tracing::info!("Successfully read {} bytes from file field", bytes.len());

                // Validate file size
                if bytes.len() > state.settings.max_file_size {
                    return Err(AppError::Validation(format!(
                        "File too large: {} bytes (max: {})",
                        bytes.len(),
                        state.settings.max_file_size
                    )));
                }

                file_data = Some(bytes);
            }
            Some(other) => {
                tracing::warn!("Ignoring unknown field: {}", other);
                // Consume the field bytes to avoid parser state errors
                let _ = field.bytes().await;
            }
            None => {
                tracing::warn!("Ignoring field without a name");
                let _ = field.bytes().await;
            }
        }
    }

    tracing::info!(
        "Finished multipart parsing: title={:?}, artist={:?}, file_size={:?}",
        title,
        artist,
        file_data.as_ref().map(|d| d.len())
    );

    // Validate required fields
    let title = title.ok_or_else(|| {
        tracing::error!("Validation failed: Title is required");
        AppError::Validation("Title is required".to_string())
    })?;

    let file_data = file_data.ok_or_else(|| {
        tracing::error!("Validation failed: Audio file is required");
        AppError::Validation("Audio file is required".to_string())
    })?;

    // Validate file type (extension and MIME type)
    let ext = validate_audio_file(file_name.as_ref(), content_type.as_ref())?;

    let track_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();

    let object_key = format!("tracks/{}/{}.{}", track_id, "original", ext);

    tracing::info!(
        "Preparing to upload file: key={}, size={} bytes, extension={}",
        object_key,
        file_data.len(),
        ext
    );
    let mut tx = state.db.begin().await?;
    // Insert track into database (status = pending)
    let track = sqlx::query_as!(
        Track,
        r#"
        INSERT INTO tracks (id, title, artist, duration_secs, object_key, status)
        VALUES ($1, $2, $3, $4, $5, 'pending')
        RETURNING id, title, artist, duration_secs, object_key,
                  status as "status: TrackStatus", created_at, updated_at
        "#,
        track_id,
        title,
        artist,
        0.0, // duration unknown until we process it
        object_key
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO fingerprint_jobs (id, track_id, status)
        VALUES ($1, $2, $3)
        "#,
        job_id,
        track.id,
        JobStatus::Queued as JobStatus
    )
    .execute(&mut *tx)
    .await?;

    tracing::info!(
        "Created track {} with job {} (status: pending)",
        track.id,
        job_id
    );

    state
        .s3
        .upload_file(&object_key, file_data.to_vec())
        .await
        .map_err(|e| AppError::Storage(format!("Failed to upload to storage: {}", e)))?;

    tx.commit().await?;
    tracing::info!("Successfully uploaded file to S3");
    Ok((StatusCode::CREATED, Json(track.into())))
}
#[derive(Deserialize)]
struct Pagination {
    page: Option<i64>,
    limit: Option<i64>,
}

async fn list_tracks(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<TrackResponse>>> {
    let page = pagination.page.unwrap_or(1).max(1);
    let limit = pagination.limit.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * limit;
    let mut tx = state.db.begin().await?;
    let tracks = sqlx::query_as!(
        Track,
        r#"
            SELECT
            id,
            title,
            artist,
            duration_secs,
            object_key,
            status as "status: TrackStatus",
            created_at,
            updated_at
            FROM tracks
            ORDER BY created_at DESC
            LIMIT $1
            OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(tracks.into_iter().map(Into::into).collect()))
}
async fn get_track(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<TrackResponse>> {
    let mut tx = state.db.begin().await?;
    let track = sqlx::query_as!(
        Track,
        r#"
            SELECT
            id,
            title,
            artist,
            duration_secs,
            object_key,
            status as "status: TrackStatus",
            created_at,
            updated_at
            FROM tracks
            WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    tx.commit().await?;
    Ok(Json(track.into()))
}
async fn get_track_url(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let mut tx = state.db.begin().await?;
    // Get just the object_key for the track
    let object_key: String = sqlx::query_scalar!(
        r#"
            SELECT object_key
            FROM tracks
            WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    let presigned_url = state.s3.get_file(&object_key).await.map_err(|e| {
        tracing::error!("Failed to generate presigned URL for track {}: {}", id, e);
        AppError::Storage(format!("Failed to generate download URL: {}", e))
    })?;

    // Return JSON with the presigned URL
    let response = serde_json::json!({
        "track_id": id,
        "url": presigned_url,
        "expires_in": "2 days"
    });
    tx.commit().await?;
    Ok(Json(response))
}
async fn delete_track(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<TrackResponse>> {
    let mut tx = state.db.begin().await?;

    let track = sqlx::query_as!(
        Track,
        r#"
            SELECT id, title, artist, duration_secs, object_key,
                   status as "status: TrackStatus", created_at, updated_at
            FROM tracks
            WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    state.s3.delete_file(&track.object_key).await.map_err(|e| {
        tracing::error!("Failed to delete S3 object {}: {}", track.object_key, e);
        AppError::Storage(format!("Failed to delete file from storage: {}", e))
    })?;

    sqlx::query!(
        r#"
            DELETE FROM tracks
            WHERE id = $1
        "#,
        id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(track.into()))
}
