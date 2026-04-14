use crate::fingerprint::CombinatorialHash;
use crate::models::tracks::Track;
use anyhow::Result;
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, FromRow)]
struct FingerprintMatchRow {
    hash: i64,
    track_id: Uuid,
    offset_ms: i32,
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub track_id: Uuid,
    pub confidence: f32,
    pub offset_ms: i32,
    pub match_count: usize,
    pub total_hashes: usize,
}
/// Find matches for a set of sample hashes
/// Algorithm:
/// step 1:  Look up each sample in the database
/// step 2:   for each match , calculate the offset = db_time - sample_time
/// step 3:   Build histogram of offsets per track
/// step 4:   Peak in histogram = match
pub async fn find_matches(
    pool: &sqlx::PgPool,
    sample_hashes: &[CombinatorialHash],
    threshold: usize, // minimum matches to consider valid
) -> Result<Vec<MatchResult>> {
    if sample_hashes.is_empty() {
        return Ok(Vec::new());
    }
    // collect all hashes values for batch lookup
    let hashes_values: Vec<i64> = sample_hashes.iter().map(|h| h.hash as i64).collect();
    // query the database for hashes
    let matches = sqlx::query_as::<_, FingerprintMatchRow>(
        r#"
        SELECT f.hash, f.track_id, f.offset_ms
        FROM fingerprints f
        WHERE f.hash = ANY($1)
        "#,
    )
    .bind(&hashes_values)
    .fetch_all(pool)
    .await?;
    // Build offset histogram per track in parallel
    // create lookup map for sample hashes (hash -> offsets)
    let mut sample_offsets: HashMap<i64, Vec<u32>> = HashMap::new();
    for h in sample_hashes {
        sample_offsets
            .entry(h.hash as i64)
            .or_default()
            .push(h.offset_ms);
    }

    let mut offset_votes: HashMap<(Uuid, i32), usize> = HashMap::new();
    for db_match in matches {
        if let Some(sample_hash_offsets) = sample_offsets.get(&db_match.hash) {
            for sample_offset in sample_hash_offsets {
                let offset = db_match.offset_ms - (*sample_offset as i32);
                let key = (db_match.track_id, offset);
                *offset_votes.entry(key).or_default() += 1;
            }
        }
    }
    // find peaks in histogram
    let mut track_best_match: HashMap<Uuid, MatchResult> = HashMap::new();
    for ((track_id, offset), count) in offset_votes {
        if count >= threshold {
            let survival_rate = (count as f32) / (sample_hashes.len() as f32);
            let confidence = (survival_rate / 0.02).min(1.0);

            let result = MatchResult {
                track_id,
                confidence,
                offset_ms: offset,
                match_count: count,
                total_hashes: sample_hashes.len(),
            };
            let entry = track_best_match.entry(track_id).or_insert(result.clone());
            if result.match_count > entry.match_count {
                *entry = result;
            }
        }
    }
    let mut results: Vec<MatchResult> = track_best_match.into_values().collect();
    results.sort_by(|a, b| b.match_count.cmp(&a.match_count));
    // Cap results to top 5
    results.truncate(5);
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::hasher::CombinatorialHash;

    #[test]
    fn test_confidence_calculation() {
        let sample_hashes_len = 50000;

        let count = 1000;
        let survival_rate = (count as f32) / (sample_hashes_len as f32);
        let confidence = (survival_rate / 0.02).min(1.0);
        assert!((confidence - 1.0).abs() < 0.001);

        let count = 100;
        let survival_rate = (count as f32) / (sample_hashes_len as f32);
        let confidence = (survival_rate / 0.02).min(1.0);
        assert!((confidence - 0.1).abs() < 0.001);

        let count = 5000;
        let survival_rate = (count as f32) / (sample_hashes_len as f32);
        let confidence = (survival_rate / 0.02).min(1.0);
        assert_eq!(confidence, 1.0);
    }
}
/// Get track metadata for match results using a batch query
pub async fn enrich_matches(
    pool: &sqlx::PgPool,
    matches: Vec<MatchResult>,
) -> Result<Vec<(Track, MatchResult)>> {
    if matches.is_empty() {
        return Ok(Vec::new());
    }
    let track_ids: Vec<Uuid> = matches.iter().map(|m| m.track_id).collect();
    let tracks = sqlx::query_as::<_, Track>(
        r#"
        SELECT id, title, artist, duration_secs, object_key, status, created_at, updated_at
        FROM tracks
        WHERE id = ANY($1)
        "#,
    )
    .bind(&track_ids)
    .fetch_all(pool)
    .await?;
    // Map tracks by ID for easy lookup
    let track_map: HashMap<Uuid, Track> = tracks.into_iter().map(|t| (t.id, t)).collect();
    let mut enriched = Vec::new();
    for match_result in matches {
        if let Some(track) = track_map.get(&match_result.track_id) {
            enriched.push((track.clone(), match_result));
        }
    }
    Ok(enriched)
}
