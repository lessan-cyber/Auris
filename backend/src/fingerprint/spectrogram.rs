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

/// Represents the spectrogram as magnitude values
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
    let fft = Arc::new(FftPlanner::new().plan_fft_forward(config.window_size));
    let hop_size = config.window_size - config.overlap;
    
    if samples.len() < config.window_size {
        return Err(anyhow::anyhow!("Audio too short for FFT window"));
    }
    
    let num_frames = (samples.len() - config.window_size) / hop_size + 1;
    let num_bins = config.window_size / 2 + 1;

    // Pre-calculate Hann window coefficients
    let hann_window: Vec<f32> = (0..config.window_size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / config.window_size as f32).cos())
        })
        .collect();

    // Process frames in parallel
    let data: Vec<f32> = (0..num_frames)
        .into_par_iter()
        .flat_map(|frame_idx| {
            let start = frame_idx * hop_size;
            let end = start + config.window_size;
            let window = &samples[start..end];

            // Local buffer for FFT
            let mut complex_buffer: Vec<Complex<f32>> = Vec::with_capacity(config.window_size);
            for i in 0..config.window_size {
                complex_buffer.push(Complex::new(window[i] * hann_window[i], 0.0));
            }

            // Perform FFT in-place
            fft.process(&mut complex_buffer);

            // Extract power (norm_sqr) for first half (positive frequencies)
            // Power is faster than magnitude (no sqrt) and standard for peaks
            complex_buffer
                .iter()
                .take(num_bins)
                .map(|c| c.norm_sqr())
                .collect::<Vec<f32>>()
        })
        .collect();

    Ok(Spectrogram {
        data,
        num_frames,
        num_bins,
        sample_rate: config.sample_rate,
        window_size: config.window_size,
        hop_size,
    })
}

pub fn bin_to_freq(bin: usize, sample_rate: u32, window_size: usize) -> f32 {
    bin as f32 * sample_rate as f32 / window_size as f32
}

pub fn frame_to_ms(frame: usize, hop_size: usize, sample_rate: u32) -> u32 {
    (frame * hop_size * 1000 / sample_rate as usize) as u32
}
