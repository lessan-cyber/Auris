use crate::AppState;
use crate::api::websocket::WsMessage;
use crate::fingerprint::{
    decode_audio, extract_peaks, generate_hashes, generate_spectrogram, hashes_to_db_records,
    peaks_to_constellation, spectrogram::SpectrogramConfig,
};
use crate::models::jobs::FingerprintJob;
use crate::models::tracks::TrackStatus;
use crate::worker::jobs::{
    fetch_next_job, mark_completed, mark_failed, mark_processing, reset_stale_jobs,
    update_heartbeat,
};
use anyhow::Result;
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
            if let Err(e) = reset_stale_jobs(&db_clone).await {
                error!("Background cleanup failed: {}", e);
            }
        }
    });

    loop {
        match fetch_next_job(&state.db).await {
            Ok(Some(job)) => {
                info!("Processing job {} for track {}", job.id, job.track_id);
                if let Err(e) = process_job(&state, &job).await {
                    warn!("❌ Job {} failed: {}", job.id, e);
                    let _ = mark_failed(&state.db, job.id, &e.to_string()).await;
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
    // Start heartbeat task in the background
    let db_clone = state.db.clone();
    let job_id = job.id;
    let (tx_heartbeat, mut rx_heartbeat) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = update_heartbeat(&db_clone, job_id).await {
                        warn!("Heartbeat failed for job {}: {}", job_id, e);
                    }
                }
                _ = &mut rx_heartbeat => {
                    break;
                }
            }
        }
    });

    // 1. Download
    let track = sqlx::query!("SELECT object_key FROM tracks WHERE id = $1", job.track_id)
        .fetch_one(&state.db)
        .await?;

    let audio_data = state.s3.download_file(&track.object_key).await?;

    // 2. Decode (blocking)
    let (samples, duration_secs) =
        tokio::task::spawn_blocking(move || decode_audio(audio_data, 8000)).await??;

    info!("Decoded: {} samples, {:.2}s", samples.len(), duration_secs);

    // 3. Spectrogram (parallel with Rayon)
    let spectrogram = tokio::task::spawn_blocking(move || {
        let config = SpectrogramConfig::default();
        generate_spectrogram(&samples, config)
    })
    .await??;

    info!(
        "Spectrogram: {} frames x {} bins",
        spectrogram.num_frames(),
        spectrogram.num_freq_bins()
    );

    // 4. Extract peaks (constellation)
    let peaks = tokio::task::spawn_blocking(move || {
        let config = SpectrogramConfig::default();
        extract_peaks(&spectrogram, &config, 0.15)
    })
    .await?;

    let constellation = peaks_to_constellation(peaks);

    info!("Constellation: {} peaks extracted", constellation.len());

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

    // Stop the heartbeat
    let _ = tx_heartbeat.send(());

    Ok(())
}

async fn process_job(state: &Arc<AppState>, job: &FingerprintJob) -> Result<()> {
    // Notification helper
    let notify = |status: &str, progress: Option<u8>, message: Option<&str>| {
        if let Some(tx_ref) = state.ws_clients.get(&job.track_id) {
            let _ = tx_ref.value().send(WsMessage {
                track_id: job.track_id,
                status: status.to_string(),
                progress,
                message: message.map(String::from),
            });
        }
    };

    // Start heartbeat task in the background
    let db_clone = state.db.clone();
    let job_id = job.id;
    let (tx_heartbeat, mut rx_heartbeat) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = update_heartbeat(&db_clone, job_id).await {
                        warn!("Heartbeat failed for job {}: {}", job_id, e);
                    }
                }
                _ = &mut rx_heartbeat => {
                    break;
                }
            }
        }
    });

    // Clain Job
    mark_processing(&state.db, job.id).await?;
    notify(
        "processing",
        Some(5),
        Some("Job claimed, starting processing"),
    );
    // update track status
    sqlx::query!(
        "UPDATE tracks SET  status = 'fingerprinting' WHERE  id= $1",
        job.track_id
    )
    .execute(&state.db)
    .await?;
    notify(
        "processing",
        Some(10),
        Some("Track status updated to fingerprinting"),
    );

    // Download from S3
    info!("Downloading track {} from S3", job.track_id);
    let track = sqlx::query!(
        "
        SELECT object_key FROM tracks WHERE id = $1",
        job.track_id
    )
    .fetch_one(&state.db)
    .await?;
    let audio_data = state.s3.download_file(&track.object_key).await?;
    notify(
        "processing",
        Some(20),
        Some("Audio downloaded from storage"),
    );
    // decode audio
    let (samples, duration_secs) =
        tokio::task::spawn_blocking(move || decode_audio(audio_data, 8000)).await??;

    info!("Decoded: {} samples, {:.2}s", samples.len(), duration_secs);
    notify("processing", Some(40), Some("Audio decoded successfully"));

    // Create a spectrogram
    let spectrogram = tokio::task::spawn_blocking(move || {
        let config = SpectrogramConfig::default();
        generate_spectrogram(&samples, config)
    })
    .await??;

    info!(
        "Spectrogram: {} frames x {} bins",
        spectrogram.num_frames(),
        spectrogram.num_freq_bins()
    );
    notify("processing", Some(50), Some("Spectrogram generated"));

    // Constellation (peaks)
    let constellation = tokio::task::spawn_blocking(move || {
        let config = SpectrogramConfig::default();
        let peaks = extract_peaks(&spectrogram, &config, 0.15);
        peaks_to_constellation(peaks)
    })
    .await?;

    info!("Constellation: {} peaks", constellation.len());
    notify(
        "processing",
        Some(70),
        Some("Peaks extracted and constellation created"),
    );

    // Hashing (combinatorial)
    let hashes =
        tokio::task::spawn_blocking(move || generate_hashes(&constellation, 300, 2000, 10)).await?;

    info!("Hash: {} combinatorial hashes", hashes.len());
    notify("processing", Some(90), Some("Fingerprint hashes generated"));

    // Bulk Insert to database
    let db_result = hashes_to_db_records(&hashes, job.track_id);

    // Use a transaction for batch insertion and track status update
    let mut tx = state.db.begin().await?;

    // insert in batches of 1000
    for chunk in db_result.chunks(1000) {
        let mut query_builder =
            sqlx::QueryBuilder::new("INSERT INTO fingerprints (hash, track_id, offset_ms) ");
        query_builder.push_values(chunk, |mut b, (hash, track_id, offset)| {
            b.push_bind(hash).push_bind(track_id).push_bind(offset);
        });

        query_builder.build().execute(&mut *tx).await?;
    }

    // update track as ready
    sqlx::query!(
        "UPDATE tracks SET status = 'ready', duration_secs = $1 WHERE id = $2",
        duration_secs,
        job.track_id
    )
    .execute(&mut *tx)
    .await?;

    // Mark job as completed (within the same transaction for atomicity)
    sqlx::query!(
        r#"
        UPDATE fingerprint_jobs
        SET status = 'completed', completed_at = NOW()
        WHERE id = $1
        "#,
        job.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Stop the heartbeat
    let _ = tx_heartbeat.send(());

    info!(
        "Track {} fingerprinted: {} hash stored",
        job.track_id,
        hashes.len()
    );
    notify(
        "completed",
        Some(100),
        Some("Fingerprinting completed successfully"),
    );
    Ok(())
}
