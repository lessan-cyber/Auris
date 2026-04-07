pub mod jobs;
pub mod mode;

use crate::AppState;
use jobs::{fetch_next_job, mark_completed, mark_failed};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub async fn run_worker(state: Arc<AppState>) {
    info!("Worker started, Pulling Jobs");
    loop {
        // fetch_next_job now atomically marks the job as 'processing'
        match jobs::fetch_next_job(&state.db).await {
            Ok(Some(job)) => {
                info!("Processing job {} for track {}", job.id, job.track_id);

                // DO THE WORK (placeholder for now)
                match process_job_stub(&state, &job).await {
                    Ok(_) => {
                        if let Err(e) = mark_completed(&state.db, job.id).await {
                            error!("Failed to mark job completed: {}", e);
                        } else {
                            info!("Job {} completed successfully", job.id);
                        }
                    }
                    Err(e) => {
                        warn!("Job {} failed: {}", job.id, e);
                        let _ = mark_failed(&state.db, job.id, &e.to_string()).await;
                    }
                }
            }
            Ok(None) => {
                // No jobs, sleep before polling again
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => {
                error!("Database error fetching job: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// Stub processor - just simulates work
/// Replace this with real fingerprinting later
async fn process_job_stub(
    state: &Arc<AppState>,
    job: &crate::models::jobs::FingerprintJob,
) -> anyhow::Result<()> {
    // Simulate CPU-intensive work
    info!("Pretending to fingerprint track {}...", job.track_id);

    // Update track status to 'fingerprinting'
    sqlx::query!(
        "UPDATE tracks SET status = 'fingerprinting' WHERE id = $1",
        job.track_id
    )
    .execute(&state.db)
    .await?;

    // Simulate work: sleep for 3 seconds (replace with real logic)
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Simulate duration detection (would come from actual audio analysis)
    let fake_duration = 180; // 3 minutes
    sqlx::query!(
        "UPDATE tracks SET duration_secs = $1, status = 'ready' WHERE id = $2",
        fake_duration,
        job.track_id
    )
    .execute(&state.db)
    .await?;

    info!(
        "Track {} ready (duration: {}s)",
        job.track_id, fake_duration
    );
    Ok(())
}
