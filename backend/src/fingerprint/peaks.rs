use super::spectrogram::{Spectrogram, SpectrogramConfig, bin_to_freq, frame_to_ms};
use rayon::prelude::*;
use std::collections::VecDeque;

/// Represents a peak in time-frequency space
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    pub frame_idx: u32, // raw frame index
    pub bin_idx: u16,   // raw FFT bin index
    pub time_ms: u32,   // derived in milliseconds, used for hashing
    pub freq_hz: u16,   // derived in hertz, used for hashing
    pub magnitude: f32,
}

/// Extract constellation points (peaks) from spectrogram
pub fn extract_peaks(
    spectrogram: &Spectrogram,
    _config: &SpectrogramConfig,
    threshold: f32,
) -> Vec<Peak> {
    let num_frames = spectrogram.num_frames();
    let num_bins = spectrogram.num_freq_bins();
    if num_frames == 0 || num_bins == 0 {
        return Vec::new();
    }
    let t_win = 5;
    let f_win = 10;

    let mut freq_max = vec![0.0f32; num_frames * num_bins];
    freq_max
        .par_chunks_exact_mut(num_bins)
        .enumerate()
        .for_each(|(t, out_row)| {
            let start = t * num_bins;
            let row = &spectrogram.data[start..start + num_bins];
            sliding_max_centered(row, f_win, out_row);
        });

    let mut window_max = vec![0.0f32; num_frames * num_bins];
    let mut col_in = vec![0.0f32; num_frames];
    let mut col_out = vec![0.0f32; num_frames];
    for f in 0..num_bins {
        for t in 0..num_frames {
            col_in[t] = freq_max[t * num_bins + f];
        }
        sliding_max_centered(&col_in, t_win, &mut col_out);
        for t in 0..num_frames {
            window_max[t * num_bins + f] = col_out[t];
        }
    }

    let mut peaks: Vec<Peak> = (0..num_frames)
        .into_par_iter()
        .flat_map(|t| {
            let mut local = Vec::new();

            for f in 10..(num_bins.saturating_sub(10)) {
                let power = spectrogram.at(t, f);

                let magnitude = power.sqrt();
                if magnitude < threshold {
                    continue;
                }

                if power >= window_max[t * num_bins + f] {
                    local.push(Peak {
                        frame_idx: t as u32,
                        bin_idx: f as u16,
                        time_ms: frame_to_ms(t, spectrogram.hop_size, spectrogram.sample_rate),
                        freq_hz: bin_to_freq(f, spectrogram.sample_rate, spectrogram.window_size)
                            as u16,
                        magnitude,
                    });
                }
            }
            local
        })
        .collect();

    filter_by_density(&mut peaks, 50);
    peaks
}

#[inline]
fn sliding_max_centered(input: &[f32], radius: usize, output: &mut [f32]) {
    debug_assert_eq!(input.len(), output.len());
    let n = input.len();
    if n == 0 {
        return;
    }

    let mut deque: VecDeque<usize> = VecDeque::with_capacity((2 * radius + 1).min(n));
    let mut right = 0usize;

    for (i, out_val) in output.iter_mut().enumerate().take(n) {
        let start = i.saturating_sub(radius);
        let end = (i + radius + 1).min(n);

        while right < end {
            let v = input[right];
            while let Some(&last_idx) = deque.back() {
                if input[last_idx] <= v {
                    deque.pop_back();
                } else {
                    break;
                }
            }
            deque.push_back(right);
            right += 1;
        }

        while deque.front().is_some_and(|&idx| idx < start) {
            deque.pop_front();
        }

        *out_val = input[*deque.front().expect("deque should never be empty")];
    }
}

/// Ensure roughly uniform density of peaks over time
fn filter_by_density(peaks: &mut Vec<Peak>, max_per_window: usize) {
    if peaks.is_empty() {
        return;
    }

    // Sort by time
    peaks.sort_by_key(|p| p.time_ms);

    let mut filtered = Vec::with_capacity(peaks.len());
    let window_size_ms = 100;

    let mut i = 0;
    while i < peaks.len() {
        let window_start_ms = peaks[i].time_ms;
        let mut j = i;

        // Find all peaks in this window
        while j < peaks.len() && peaks[j].time_ms < window_start_ms + window_size_ms {
            j += 1;
        }

        // Extract and sort by magnitude descending
        let mut window_slice = peaks[i..j].to_vec();
        window_slice.sort_by(|a, b| b.magnitude.total_cmp(&a.magnitude));

        // Keep strongest N
        for p in window_slice.iter().take(max_per_window) {
            filtered.push(*p);
        }

        i = j;
    }

    *peaks = filtered;
}

// Convert peaks to (time_ms, freq_hz) for hashing
pub fn peaks_to_constellation(peaks: Vec<Peak>) -> Vec<(u32, u16)> {
    peaks.into_iter().map(|p| (p.time_ms, p.freq_hz)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::spectrogram::Spectrogram;

    #[test]
    fn test_sliding_max_centered() {
        let input = vec![1.0, 2.0, 1.5, 3.0, 2.5, 1.0];
        let mut output = vec![0.0; input.len()];
        sliding_max_centered(&input, 1, &mut output);
        // Radius 1 means window of 3: [1,2,1.5] -> 2.0, [2,1.5,3] -> 3.0, etc.
        assert_eq!(output, vec![2.0, 2.0, 3.0, 3.0, 3.0, 2.5]);
    }

    #[test]
    fn test_extract_peaks_simple() {
        let mut data = vec![0.0f32; 100 * 100];
        // Create a single clear peak at (50, 50)
        data[50 * 100 + 50] = 100.0;
        
        let spectrogram = Spectrogram {
            data,
            num_frames: 100,
            num_bins: 100,
            sample_rate: 8000,
            window_size: 1024,
            hop_size: 512,
        };

        let peaks = extract_peaks(&spectrogram, &SpectrogramConfig::default(), 1.0);
        assert!(!peaks.is_empty());
        let peak = &peaks[0];
        assert_eq!(peak.frame_idx, 50);
        assert_eq!(peak.bin_idx, 50);
    }

    #[test]
    fn test_filter_by_density() {
        let mut peaks = Vec::new();
        // Create 100 peaks in the same 100ms window
        for i in 0..100 {
            peaks.push(Peak {
                frame_idx: 0,
                bin_idx: i as u16,
                time_ms: 10,
                freq_hz: i as u16 * 40,
                magnitude: i as f32, // increasing magnitude
            });
        }

        filter_by_density(&mut peaks, 10);
        // Should only keep top 10 strongest
        assert_eq!(peaks.len(), 10);
        assert!(peaks.iter().all(|p| p.magnitude >= 90.0));
    }
}
