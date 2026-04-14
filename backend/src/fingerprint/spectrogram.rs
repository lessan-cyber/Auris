use anyhow::Result;
use num_complex::Complex;
use rayon::prelude::*;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct SpectrogramConfig {
    pub sample_rate: u32,
    pub window_size: usize, // FFT size (e.g., 1024)
    pub overlap: usize,
}
impl Default for SpectrogramConfig {
    fn default() -> Self {
        Self {
            sample_rate: 8000,
            window_size: 1024,
            overlap: 512,
        }
    }
}
///  Represents the spectrogram as power values (squared magnitude)
/// Flattened into a single Vec for cache locality: [time * freq_bins + freq]
pub struct Spectrogram {
    pub data: Vec<f32>,
    pub num_frames: usize,
    pub num_bins: usize,
    pub sample_rate: u32,
    pub window_size: usize,
    pub hop_size: usize,
}
impl Spectrogram {
    #[inline(always)]
    pub fn at(&self, time: usize, freq: usize) -> f32 {
        self.data[time * self.num_bins + freq]
    }
    pub fn num_frames(&self) -> usize {
        self.num_frames
    }
    pub fn num_freq_bins(&self) -> usize {
        self.num_bins
    }
}
/// Generate spectrogram from audio samples
/// Uses Rayon for parallel FFT processing and pre-calculates the Hann window.
pub fn generate_spectrogram(samples: &[f32], config: SpectrogramConfig) -> Result<Spectrogram> {
    // Validate config invariants
    if config.window_size == 0 {
        return Err(anyhow::anyhow!("window_size must be greater than 0"));
    }
    if config.overlap >= config.window_size {
        return Err(anyhow::anyhow!("overlap must be less than window_size"));
    }
    if config.sample_rate == 0 {
        return Err(anyhow::anyhow!("sample_rate must be greater than 0"));
    }
    let window_size = config.window_size;
    let hop_size = window_size - config.overlap;
    let fft = Arc::new(FftPlanner::new().plan_fft_forward(window_size));

    if samples.len() < window_size {
        return Err(anyhow::anyhow!("Audio too short for FFT window"));
    }
    let num_frames = (samples.len() - window_size) / hop_size + 1;
    let num_bins = window_size / 2 + 1;
    // Pre-calculate Hann window coefficients
    let hann_window: Vec<f32> = (0..window_size)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / window_size as f32).cos()))
        .collect();
    // Pre-allocate the entire output data vector to avoid allocator churn
    let mut data = vec![0.0f32; num_frames * num_bins];
    // Process frames in parallel using chunks_exact_mut and for_each_init to reuse buffers
    data.par_chunks_exact_mut(num_bins)
        .enumerate()
        .for_each_init(
            || vec![Complex::new(0.0f32, 0.0f32); window_size], // Per-thread scratch buffer
            |complex_buffer, (frame_idx, output_frame)| {
                let start = frame_idx * hop_size;
                let end = start + window_size;
                let window = &samples[start..end];
                for i in 0..window_size {
                    complex_buffer[i].re = window[i] * hann_window[i];
                    complex_buffer[i].im = 0.0;
                }
                // Perform FFT in-place
                fft.process(complex_buffer);
                // Extract power (norm_sqr) for first half (positive frequencies)
                for i in 0..num_bins {
                    output_frame[i] = complex_buffer[i].norm_sqr();
                }
            },
        );
    Ok(Spectrogram {
        data,
        num_frames,
        num_bins,
        sample_rate: config.sample_rate,
        window_size,
        hop_size,
    })
}
pub fn bin_to_freq(bin: usize, sample_rate: u32, window_size: usize) -> f32 {
    bin as f32 * sample_rate as f32 / window_size as f32
}
pub fn frame_to_ms(frame: usize, hop_size: usize, sample_rate: u32) -> u32 {
    (frame * hop_size * 1000 / sample_rate as usize) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_spectrogram_sine_wave() {
        let sample_rate = 8000;
        let freq = 440.0;
        let duration_secs = 1.0;
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        
        // Generate a 440Hz sine wave
        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect();

        let config = SpectrogramConfig {
            sample_rate,
            window_size: 1024,
            overlap: 512,
        };

        let spectrogram = generate_spectrogram(&samples, config).expect("Spectrogram generation failed");
        assert!(spectrogram.num_frames > 0);
        assert_eq!(spectrogram.num_bins, 513);

        // Find the bin with the most power in the first frame
        let mut max_power = 0.0;
        let mut max_bin = 0;
        for bin in 0..spectrogram.num_bins {
            let power = spectrogram.at(0, bin);
            if power > max_power {
                max_power = power;
                max_bin = bin;
            }
        }

        let detected_freq = bin_to_freq(max_bin, sample_rate, 1024);
        // 440Hz at 8000Hz SR and 1024 FFT size: bin = 440 * 1024 / 8000 = 56.32 -> bin 56 or 57
        assert!((detected_freq - freq).abs() < 10.0, "Detected frequency {} should be near {}Hz", detected_freq, freq);
    }

    #[test]
    fn test_bin_to_freq() {
        assert_eq!(bin_to_freq(0, 8000, 1024), 0.0);
        assert_eq!(bin_to_freq(512, 8000, 1024), 4000.0);
    }
}
