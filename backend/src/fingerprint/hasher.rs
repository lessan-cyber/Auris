#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CombinatorialHash {
    /// the actual hash value
    pub hash: u32,
    /// the absolute time of an anchor point in the track (ms)
    pub offset_ms: u32,
}
/// Generate combinatorial hashes from a list of peaks (constellation points)
/// This follows the Shazam-style algorithm:
/// 1. Pick an anchor peak
/// 2. Pick target peaks in a "target zone" relative to the anchor
/// 3. Create a hash of (freq_anchor, freq_target, time_delta)
pub fn generate_hashes(
    constellation: &[(u32, u16)],
    target_zone_time_ms: u32,
    target_zone_freq_hz: u16,
    fan_out: usize,
) -> Vec<CombinatorialHash> {
    if constellation.len() < 2 || fan_out == 0 {
        return Vec::new();
    }

    let mut points: Vec<(u32, u16)> = constellation.to_vec();
    if points.windows(2).any(|w| w[0].0 > w[1].0) {
        points.sort_unstable_by_key(|p| p.0);
    }

    let mut hashes = Vec::with_capacity(points.len().saturating_mul(fan_out));
    let mut end = 1usize;

    for i in 0..points.len() {
        let (t1, f1) = points[i];
        let t_max = t1.saturating_add(target_zone_time_ms);
        let f_min = f1.saturating_sub(target_zone_freq_hz);
        let f_max = f1.saturating_add(target_zone_freq_hz);

        if end < i + 1 {
            end = i + 1;
        }
        while end < points.len() && points[end].0 <= t_max {
            end += 1;
        }

        let mut pairs = 0usize;
        for &(t2, f2) in &points[(i + 1)..end] {
            if f2 < f_min || f2 > f_max {
                continue;
            }

            hashes.push(CombinatorialHash {
                hash: pack_hash(f1 as u32, f2 as u32, t2 - t1),
                offset_ms: t1,
            });

            pairs += 1;
            if pairs >= fan_out {
                break;
            }
        }
    }

    hashes
}
/// Pack frequency and time values into single u32 hash
/// Format: [10 bits f1][10 bits f2][12 bits delta_t]
fn pack_hash(f1: u32, f2: u32, delta_t: u32) -> u32 {
    // Quantize frequencies to 10 bits (0-1023)
    // Assuming max frequency ~4000Hz, divide by 4 to fit in 10 bits
    let f1_quant = (f1 / 4).min(1023);
    let f2_quant = (f2 / 4).min(1023);
    // Delta_t limited to 12 bits (0-4095 ms)
    let dt_clamped = delta_t.min(4095);
    // Pack: f1 in upper bits, f2 middle, dt lower
    (f1_quant << 22) | (f2_quant << 12) | dt_clamped
}
/// Unpack hash for debugging/verification (optional)
pub fn unpack_hash(hash: u32) -> (u32, u32, u32) {
    let f1 = (hash >> 22) & 0x3FF;
    let f2 = (hash >> 12) & 0x3FF;
    let dt = hash & 0xFFF;
    (f1 * 4, f2 * 4, dt) // Multiply back by 4 for approximate freq
}
/// Batch insert helper for database
pub fn hashes_to_db_records(
    hashes: &[CombinatorialHash],
    track_id: uuid::Uuid,
) -> Vec<(i64, uuid::Uuid, i32)> {
    hashes
        .iter()
        .map(|h| (h.hash as i64, track_id, h.offset_ms as i32))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_pack_unpack() {
        let f1 = 440; // A4 note
        let f2 = 880; // A5 note
        let dt = 50; // 50ms

        let packed = pack_hash(f1, f2, dt);
        let (uf1, uf2, udt) = unpack_hash(packed);

        // Should be close due to quantization (divide by 4)
        assert!((uf1 as i32 - f1 as i32).abs() <= 4);
        assert!((uf2 as i32 - f2 as i32).abs() <= 4);
        assert_eq!(udt, dt);
    }

    #[test]
    fn test_generate_hashes() {
        // Simple test constellation
        let points = vec![
            (0, 100),   // t=0ms, f=100Hz
            (10, 200),  // t=10ms, f=200Hz
            (20, 150),  // t=20ms, f=150Hz
            (100, 300), // t=100ms, f=300Hz (outside time zone)
        ];

        let hashes = generate_hashes(&points, 50, 120, 6);
        eprintln!("{:?}", hashes);

        // Should have hashes for pairs within 50ms
        assert!(!hashes.is_empty());

        // First anchor (0ms) should pair with 10ms and 20ms points
        let first_anchor_hashes: Vec<_> = hashes.iter().filter(|h| h.offset_ms == 0).collect();
        eprintln!("{:?}", first_anchor_hashes);
        assert_eq!(first_anchor_hashes.len(), 2);
    }
}
