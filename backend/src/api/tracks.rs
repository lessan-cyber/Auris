use crate::AppState;
use crate::errors::{AppError, Result};
use crate::models::jobs::JobStatus;
use crate::models::tracks::{
    ListTracksResponse, Track, TrackResponse, TrackStatus, UpdateTrackRequest,
};
use crate::utils::file_hash::{check_file_hash_exists, generate_file_hash};
use crate::utils::file_validation::validate_audio_file;
use axum::extract::Path;
use axum::{
    Json, Router,
    extract::{Multipart, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
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
        .route("/{id}", patch(update_track))
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
    let file_data_for_hash = file_data.clone();
    let file_hash =
        tokio::task::spawn_blocking(move || generate_file_hash(&file_data_for_hash[..])).await?;

    // Validate file type (extension and MIME type)
    let ext = validate_audio_file(file_name.as_ref(), content_type.as_ref())?;

    let track_id = Uuid::now_v7();
    let job_id = Uuid::now_v7();

    let object_key = format!("tracks/{}/{}.{}", track_id, "original", ext);

    tracing::info!(
        "Preparing to upload file: key={}, size={} bytes, extension={}",
        object_key,
        file_data.len(),
        ext
    );

    // 1. Transaction 1: Create the track record and commit immediately.
    // This releases the DB connection back to the pool during the slow S3 upload.
    let mut tx = state.db.begin().await?;
    let track_opt = sqlx::query_as!(
        Track,
        r#"
        INSERT INTO tracks (id, title, artist, duration_secs, object_key, file_hash, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'pending')
        ON CONFLICT (file_hash) DO NOTHING
        RETURNING id, title, artist, duration_secs, object_key, file_hash,
                  status as "status: TrackStatus", created_at, updated_at
        "#,
        track_id,
        title,
        artist,
        0.0,
        object_key,
        file_hash
    )
    .fetch_optional(&mut *tx)
    .await?;

    if track_opt.is_none() {
        tx.rollback().await?;
        let existing_track = check_file_hash_exists(&state.db, &file_hash)
            .await?
            .ok_or_else(|| AppError::Internal("Duplicate detected but not found".to_string()))?;

        return Ok((
            StatusCode::CONFLICT,
            Json(TrackResponse::from(existing_track)),
        ));
    }

    let track = track_opt.unwrap();
    tx.commit().await?;
    tracing::info!("Track metadata committed (ID: {})", track.id);

    // 2. S3 Upload (No DB connection held)
    let upload_result = state.s3.upload_file(&object_key, file_data.clone()).await;

    if let Err(e) = upload_result {
        tracing::error!("S3 upload failed for track {}: {}", track.id, e);
        // Mark as error in a fresh query
        sqlx::query!("UPDATE tracks SET status = 'error' WHERE id = $1", track.id)
            .execute(&state.db)
            .await?;

        return Err(AppError::Storage(format!(
            "Failed to upload to storage: {}",
            e
        )));
    }

    // 3. Create the processing job now that the file is safely in S3
    sqlx::query!(
        r#"
        INSERT INTO fingerprint_jobs (id, track_id, status)
        VALUES ($1, $2, $3)
        "#,
        job_id,
        track.id,
        JobStatus::Queued as JobStatus
    )
    .execute(&state.db)
    .await?;

    tracing::info!(
        "Successfully uploaded track {} and queued job {}",
        track.id,
        job_id
    );

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
) -> Result<Json<ListTracksResponse>> {
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
            file_hash,
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
    let total = sqlx::query!(
        r#"
            SELECT COUNT(*)
            FROM tracks
        "#
    )
    .fetch_one(&mut *tx)
    .await?
    .count;

    tx.commit().await?;
    Ok(Json(ListTracksResponse {
        tracks: tracks.into_iter().map(Into::into).collect(),
        total_count: total.unwrap_or(0),
    }))
    //Ok(Json(tracks.into_iter().map(Into::into).collect()))
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
            file_hash,
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
            SELECT id, title, artist, duration_secs, object_key, file_hash,
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

    Ok(Json(track.into()))
}

async fn update_track(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTrackRequest>,
) -> Result<Json<TrackResponse>> {
    if payload.title.is_none() && payload.artist.is_none() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let mut query_builder = sqlx::QueryBuilder::new("UPDATE tracks SET ");
    let mut separated = query_builder.separated(", ");

    if let Some(title) = payload.title {
        separated.push("title = ");
        separated.push_bind_unseparated(title);
    }

    if let Some(artist) = payload.artist {
        separated.push("artist = ");
        separated.push_bind_unseparated(artist);
    }

    // Always update updated_at
    separated.push("updated_at = NOW() ");

    query_builder.push(" WHERE id = ");
    query_builder.push_bind(id);
    query_builder.push(
        " RETURNING id, title, artist, duration_secs, object_key, file_hash,created_at, updated_at",
    );

    let track = query_builder
        .build_query_as::<Track>()
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(track.into()))
}
