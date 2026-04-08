use rayon::prelude::*;
use super::spectrogram::{Spectrogram, SpectrogramConfig, bin_to_freq, frame_to_ms};

/// Represents a peak in time-frequency space
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    pub time_ms: u32,
    pub freq_hz: u16,
    pub magnitude: f32,
}

/// Extract constellation points (peaks) from spectrogram
/// Uses Rayon for parallel local maximum detection.
pub fn extract_peaks(
    spectrogram: &Spectrogram,
    _config: &SpectrogramConfig,
    threshold: f32,
) -> Vec<Peak> {
    let num_frames = spectrogram.num_frames();
    let num_bins = spectrogram.num_freq_bins();

    // Neighborhood size for local maxima detection
    let time_neighborhood = 5; // +/- 5 frames
    let freq_neighborhood = 10; // +/- 10 bins

    // Parallelize the frame processing loop
    let mut peaks: Vec<Peak> = (0..num_frames)
        .into_par_iter()
        .flat_map(|t| {
            let mut local_peaks = Vec::new();
            // Skip DC and Nyquist edge cases
            for f in 10..(num_bins.saturating_sub(10)) {
                let mag = spectrogram.at(t, f);

                if mag < threshold {
                    continue;
                }

                if is_local_maximum(spectrogram, t, f, time_neighborhood, freq_neighborhood, mag) {
                    local_peaks.push(Peak {
                        time_ms: frame_to_ms(t, spectrogram.hop_size, spectrogram.sample_rate),
                        freq_hz: bin_to_freq(f, spectrogram.sample_rate, spectrogram.window_size) as u16,
                        magnitude: mag,
                    });
                }
            }
            local_peaks
        })
        .collect();

    // Density filtering (ensure uniform coverage)
    filter_by_density(&mut peaks, 50);

    peaks
}

#[inline(always)]
fn is_local_maximum(
    spectrogram: &Spectrogram,
    t: usize,
    f: usize,
    t_window: usize,
    f_window: usize,
    val: f32,
) -> bool {
    let t_start = t.saturating_sub(t_window);
    let t_end = (t + t_window + 1).min(spectrogram.num_frames());
    let f_start = f.saturating_sub(f_window);
    let f_end = (f + f_window + 1).min(spectrogram.num_freq_bins());

    for check_t in t_start..t_end {
        for check_f in f_start..f_end {
            if check_t == t && check_f == f {
                continue;
            }
            if spectrogram.at(check_t, check_f) >= val {
                return false;
            }
        }
    }
    true
}

/// Ensure roughly uniform density of peaks over time
fn filter_by_density(peaks: &mut Vec<Peak>, max_per_window: usize) {
    if peaks.is_empty() { return; }

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

pub fn peaks_to_constellation(peaks: Vec<Peak>) -> Vec<(u32, u16)> {
    peaks.into_iter().map(|p| (p.time_ms, p.freq_hz)).collect()
}
