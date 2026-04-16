use crate::models::tracks::{Track, TrackStatus};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

pub fn generate_file_hash(file: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(file);
    let hash_result = hasher.finalize();
    
    let mut s = String::with_capacity(hash_result.len() * 2);
    for b in hash_result {
        use std::fmt::Write;
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

/// Check if a file hash already exists in the database
/// Returns Ok(Some(track)) if duplicate found, Ok(None) if not found, or Err if database error
pub async fn check_file_hash_exists(
    db: &PgPool,
    file_hash: &str,
) -> Result<Option<Track>, sqlx::Error> {
    sqlx::query_as!(
        crate::models::tracks::Track,
        r#"
        SELECT id, title, artist, duration_secs, object_key, file_hash,
               status as "status: TrackStatus", created_at, updated_at
        FROM tracks
        WHERE file_hash = $1
        LIMIT 1
        "#,
        file_hash
    )
    .fetch_optional(db)
    .await
}
