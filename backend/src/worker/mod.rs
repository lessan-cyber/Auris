pub mod jobs;
pub mod mode;
use crate::AppState;
use crate::fingerprint::decode::decode_audio;
use crate::fingerprint::{
    extract_peaks, generate_spectrogram, peaks_to_constellation, spectrogram::SpectrogramConfig,
};
use crate::models::jobs::FingerprintJob;
use anyhow::Result;
use jobs::{fetch_next_job, mark_completed, mark_failed};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub async fn run_worker(state: Arc<AppState>) -> Result<()> {
    info!("Worker started, Pulling Jobs");
    // Spawn periodic cleanup for stale jobs
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = jobs::reset_stale_jobs(&db_clone).await {
                error!("Background cleanup failed: {}", e);
            }
        }
    });

    loop {
        match fetch_next_job(&state.db).await {
            Ok(Some(job)) => {
                info!("Processing job {} for track {}", job.id, job.track_id);

                match process_job_with_spectrogram(&state, &job).await {
                    Ok(_) => {
                        if let Err(e) = mark_completed(&state.db, job.id).await {
                            error!("Failed to mark job completed: {}", e);
                        } else {
                            info!("Job {} completed successfully", job.id);
                        }
                    }
                    Err(e) => {
                        warn!("Job {} failed: {}", job.id, e);
                        if let Err(mark_err) = mark_failed(&state.db, job.id, &e.to_string()).await {
                            error!("Failed to mark job {} as failed: {}", job.id, mark_err);
                        }
                    }
                }
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => {
                error!("Database error fetching job: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn process_job_with_spectrogram(state: &Arc<AppState>, job: &FingerprintJob) -> Result<()> {
    // 1. Download
    let track = sqlx::query!("SELECT object_key FROM tracks WHERE id = $1", job.track_id)
        .fetch_one(&state.db)
        .await?;

    let audio_data = state.s3.download_file(&track.object_key).await?;

    // 2. Decode (blocking)
    let (samples, duration_secs) =
        tokio::task::spawn_blocking(move || decode_audio(audio_data, 8000)).await??;

    info!(
        "🎵 Decoded: {} samples, {:.2}s",
        samples.len(),
        duration_secs
    );

    // 3. Spectrogram (parallel with Rayon)
    let spectrogram = tokio::task::spawn_blocking(move || {
        let config = SpectrogramConfig::default();
        generate_spectrogram(&samples, config)
    })
    .await??;

    info!(
        "📊 Spectrogram: {} frames x {} bins",
        spectrogram.num_frames(),
        spectrogram.num_freq_bins()
    );

    // 4. Extract peaks (constellation)
    let peaks = tokio::task::spawn_blocking(move || {
        let config = SpectrogramConfig::default();
        extract_peaks(&spectrogram, &config, 0.2)
    })
    .await?;

    let constellation = peaks_to_constellation(peaks);

    info!("⭐ Constellation: {} peaks extracted", constellation.len());

    // 5. Update track status and duration (preserve f64 precision)
    let mut tx = state.db.begin().await?;
    sqlx::query!(
        "UPDATE tracks SET duration_secs = $1, status = 'ready' WHERE id = $2",
        duration_secs,
        job.track_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(())
}
