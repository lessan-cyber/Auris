use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents a song/track in the database.
/// Maps to the `tracks` table in the database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Track {
    pub id: Uuid,
    pub title: String,
    pub artist: Option<String>,
    pub duration_secs: f64,
    pub object_key: String,
    pub status: TrackStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "track_status", rename_all = "snake_case")]
pub enum TrackStatus {
    Pending,
    Fingerprinting,
    Ready,
    Error,
}

/// Request Body for registering a new track.
#[derive(Debug, Deserialize)]
pub struct RegisterTrackRequest {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
}

/// Response for registering a new track.
#[derive(Debug, Serialize)]
pub struct TrackResponse {
    pub id: Uuid,
    pub title: String,
    pub artist: Option<String>,
    pub status: String,
    pub duration_secs: f64,
    pub created_at: DateTime<Utc>,
}

impl From<Track> for TrackResponse {
    fn from(track: Track) -> Self {
        Self {
            id: track.id,
            title: track.title,
            artist: track.artist,
            status: format!("{:?}", track.status).to_lowercase(),
            duration_secs: track.duration_secs,
            created_at: track.created_at,
        }
    }
}
