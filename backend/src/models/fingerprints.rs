use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
/// Database record for a single fingerprint hash
/// Maps to the `fingerprints` table in the database.
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Fingerprint {
    pub hash: i32,      // Stored as i32 in Postgres, but represents u32
    pub track_id: Uuid, // Stored as UUID in Postgres
    pub offset_ms: i32,
    pub created_at: DateTime<Utc>,
}

/// In-Memory representation before database insertion.
/// uses unsigned type for calculations , convert to i32 for DB
#[derive(Debug, Clone, Copy)]
pub struct HashRecord {
    pub hash: u32,
    pub offset_ms: u32,
}

impl HashRecord {
    /// Convert to database-compatible types
    pub fn to_db_record(self, track_id: Uuid) -> (i32, Uuid, i32) {
        (self.hash as i32, track_id, self.offset_ms as i32)
    }
}

/// Internal representation of a constellation point
/// time and frequency coordinates extracted from the spectrogram
#[derive(Debug, Clone, Copy)]
pub struct ConstellationPoint {
    pub time_ms: u32,
    pub freq_hz: u32,
}

/// configuration for fingerprint algorithm
#[derive(Debug, Clone)]
pub struct FingerprintConfig {
    /// Target sample rate (8khz is the default)
    pub sample_rate_hz: u32,
    /// FFT window size in samples
    pub fft_window_size: u32,
    /// Overlap between FFT windows in samples
    pub overlap: u32,
    /// Minimum magnitude to consider as a peak
    pub peak_threshold: f32,
    /// how many points to pair with each anchor (fan-out factor F)
    pub fan_out: usize,
    /// Target zone : time range in ms
    pub target_zone_start_ms: u32,
    pub target_zone_end_ms: u32,
    /// Target zone : frequency range in Hz
    pub target_zone_start_hz: u16,
    pub target_zone_end_hz: u16,
}
impl Default for FingerprintConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 8000,
            fft_window_size: 1024,
            overlap: 512,
            peak_threshold: 0.2,
            fan_out: 10,
            target_zone_start_ms: 10,
            target_zone_end_ms: 100,
            target_zone_start_hz: 0,
            target_zone_end_hz: 4000,
        }
    }
}
