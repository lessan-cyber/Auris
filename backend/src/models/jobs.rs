use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Background Job for fingerprinting a track
/// Maps to the fingerprint_jobs table
#[derive(Debug, Clone, FromRow)]
pub struct FingerprintJob {
    pub id: Uuid,
    pub track_id: Uuid,
    pub status: JobStatus,
    pub attempts: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "job_status", rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Processing,
    Completed,
    Failed,
}

/// Response for Job status endpoint
#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub id: Uuid,
    pub track_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<FingerprintJob> for JobResponse {
    fn from(job: FingerprintJob) -> Self {
        JobResponse {
            id: job.id,
            track_id: job.track_id,
            status: format!("{:?}", job.status).to_lowercase(),
            created_at: job.created_at,
            completed_at: job.completed_at,
        }
    }
}
