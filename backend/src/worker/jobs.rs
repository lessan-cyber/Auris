use crate::models::jobs::{FingerprintJob, JobStatus};
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;
/// Automatically fetch and claim the next queued job atomically.
/// Uses UPDATE ... RETURNING with SKIP LOCKED to prevent double-processing.
pub async fn fetch_next_job(pool: &PgPool) -> Result<Option<FingerprintJob>> {
    let job = sqlx::query_as!(
        FingerprintJob,
        r#"
        UPDATE fingerprint_jobs
        SET status = 'processing', attempts = attempts + 1
        WHERE id = (
            SELECT id FROM fingerprint_jobs
            WHERE status = 'queued'
            ORDER BY created_at
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING
            id,
            track_id,
            status as "status: JobStatus",
            attempts,
            error_message,
            created_at,
            updated_at,
            completed_at
        "#
    )
    .fetch_optional(pool)
    .await?;

    Ok(job)
}

/// Resets stale jobs that have been in 'processing' state for too long.
/// This handles cases where a worker might have crashed without updating the job status.
pub async fn reset_stale_jobs(pool: &PgPool) -> Result<()> {
    let mut tx = pool.begin().await?;

    // 1. Mark jobs with too many attempts (>= 3) as terminally failed
    let failed = sqlx::query!(
        r#"
        WITH stale_jobs AS (
            UPDATE fingerprint_jobs
            SET status = 'failed', error_message = 'Max retries exceeded after timeout'
            WHERE status = 'processing'
            AND updated_at < NOW() - INTERVAL '10 minutes'
            AND attempts >= 3
            RETURNING track_id
        )
        UPDATE tracks
        SET status = 'error'
        WHERE id IN (SELECT track_id FROM stale_jobs)
        "#
    )
    .execute(&mut *tx)
    .await?;

    if failed.rows_affected() > 0 {
        tracing::warn!(
            "Marked {} stale jobs as terminally failed",
            failed.rows_affected()
        );
    }

    // 2. Reset others back to queued
    let reset = sqlx::query!(
        r#"
        UPDATE fingerprint_jobs
        SET status = 'queued', error_message = 'Reset after timeout'
        WHERE status = 'processing'
        AND updated_at < NOW() - INTERVAL '10 minutes'
        AND attempts < 3
        "#
    )
    .execute(&mut *tx)
    .await?;

    if reset.rows_affected() > 0 {
        tracing::info!("Reset {} stale jobs back to queued", reset.rows_affected());
    }

    tx.commit().await?;
    Ok(())
}

/// Mark job as completed
pub async fn mark_completed(pool: &PgPool, job_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query!(
        r#"
        UPDATE fingerprint_jobs
        SET status = 'completed', completed_at = NOW()
        WHERE id = $1
        "#,
        job_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Refreshes the lease on a job by updating the updated_at timestamp
pub async fn update_heartbeat(pool: &PgPool, job_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE fingerprint_jobs SET updated_at = NOW() WHERE id = $1",
        job_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Marks a job as failed
pub async fn mark_failed(pool: &PgPool, job_id: Uuid, error_message: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query!(
        r#"
        UPDATE fingerprint_jobs
        SET status = 'failed', error_message = $2
        WHERE id = $1
        "#,
        job_id,
        error_message
    )
    .execute(&mut *tx)
    .await?;
    // also mark track status as error
    sqlx::query!(
        r#"
        UPDATE tracks
        SET status = 'error'
        WHERE id = (SELECT track_id FROM fingerprint_jobs WHERE id = $1)
        "#,
        job_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn mark_processing(pool: &PgPool, job_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query!(
        r#"
        UPDATE fingerprint_jobs
        SET status = 'processing',updated_at = NOW()

        WHERE id = $1
        "#,
        job_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
