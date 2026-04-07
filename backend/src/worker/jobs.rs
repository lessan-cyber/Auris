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
            completed_at
        "#
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(job)
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
