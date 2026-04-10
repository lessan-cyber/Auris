use crate::errors::{AppError, Result};
use crate::{
    AppState,
    fingerprint::{
        CombinatorialHash, SpectrogramConfig, decode_audio, extract_peaks, generate_hashes,
        generate_spectrogram,
        matcher::{enrich_matches, find_matches},
        peaks_to_constellation,
    },
    models::tracks::TrackResponse,
};
use axum::{
    Router,
    extract::{Multipart, State},
    response::Json,
    routing::post,
};
use rayon::prelude::*;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
#[derive(Serialize)]
pub struct IdentifyResponse {
    pub matches: Vec<MatchDetail>,
    pub query_duration_ms: u32,
    pub sample_duration_secs: f64,
}
#[derive(Serialize)]
pub struct MatchDetail {
    pub track: TrackResponse,
    pub confidence: f32,
    pub match_count: usize,
    pub offset_secs: f32,
}
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", post(identify_track))
}
async fn identify_track(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<IdentifyResponse>> {
    let start_time = std::time::Instant::now();
    // Extract audio file from multipart
    let mut audio_data = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart: {}", e)))?
    {
        if field.name() == Some("file") {
            audio_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Failed to read file: {}", e)))?,
            );
            break;
        }
    }
    let audio_data =
        audio_data.ok_or_else(|| AppError::Validation("Audio file required".to_string()))?;
    // Process sample (same pipeline as ingestion, but we don't store)
    let (sample_hashes, sample_duration) = tokio::task::spawn_blocking(move || {
        let (samples, duration) = decode_audio(audio_data.to_vec(), 8000)?;

        let config = SpectrogramConfig::default();
        let spectrogram = generate_spectrogram(&samples, config)?;
        let peaks = extract_peaks(&spectrogram, &config, 0.15);
        let constellation = peaks_to_constellation(peaks);
        let hashes = generate_hashes(&constellation, 300, 2000, 10);
        Ok::<(Vec<CombinatorialHash>, f64), anyhow::Error>((hashes, duration))
    })
    .await??;
    tracing::info!(
        "Sample processed: {}s, {} hashes",
        sample_duration,
        sample_hashes.len()
    );
    // Query database for matches
    // Threshold: need at least 10 matching hashes to be considered
    let matches = find_matches(&state.db, &sample_hashes, 10).await?;
    // Enrich with track metadata
    let enriched = enrich_matches(&state.db, matches).await?;
    let query_duration = start_time.elapsed().as_millis() as u32;
    // Build response
    let match_details: Vec<MatchDetail> = enriched
        .into_par_iter()
        .map(|(track, result)| MatchDetail {
            track: track.into(),
            confidence: result.confidence.min(1.0),
            match_count: result.match_count,
            offset_secs: result.offset_ms as f32 / 1000.0,
        })
        .collect();
    Ok(Json(IdentifyResponse {
        matches: match_details,
        query_duration_ms: query_duration,
        sample_duration_secs: sample_duration,
    }))
}
