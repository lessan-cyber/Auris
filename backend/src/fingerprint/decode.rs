use anyhow::Result;
use rayon::prelude::*;
use std::io::Cursor;
use symphonia::core::audio::{AudioBuffer, Channels, SampleBuffer, SignalSpec};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decodes audio file to 8kHz mono f32 samples
/// Returns (samples, duration_seconds)
pub fn decode_audio(data: Vec<u8>, target_sample_rate: u32) -> Result<(Vec<f32>, f64)> {
    let start = std::time::Instant::now();
    let mss = create_media_source(data);
    let mut format = probe_format(mss)?;
    let (track_id, decoder) = create_decoder(&mut format)?;

    let decode_start = std::time::Instant::now();
    let (samples, sample_rate, channels) = decode_packets(&mut format, track_id, decoder)?;
    let decode_elapsed = decode_start.elapsed();
    
    let mono_start = std::time::Instant::now();
    let mono_samples = convert_to_mono(samples, channels);
    let mono_elapsed = mono_start.elapsed();
    
    let resample_start = std::time::Instant::now();
    let resampled = resample_to_target(mono_samples, sample_rate, target_sample_rate);
    let resample_elapsed = resample_start.elapsed();

    let total_elapsed = start.elapsed();
    tracing::info!("      - Symphonia decode: {:?}", decode_elapsed);
    tracing::info!("      - Mono conversion:  {:?}", mono_elapsed);
    tracing::info!("      - Resampling:       {:?}", resample_elapsed);
    tracing::info!("      - Total decode_audio: {:?}", total_elapsed);

    let duration_secs = resampled.len() as f64 / target_sample_rate as f64;
    Ok((resampled, duration_secs))
}

/// Create media source stream from audio data
fn create_media_source(data: Vec<u8>) -> MediaSourceStream {
    MediaSourceStream::new(Box::new(Cursor::new(data)), Default::default())
}

/// Probe the audio format and return the format reader
fn probe_format(mss: MediaSourceStream) -> Result<Box<dyn FormatReader>> {
    let hint = Hint::new();
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| anyhow::anyhow!("Failed to probe format: {}", e))?;

    Ok(probed.format)
}

/// Find audio track and create decoder
fn create_decoder(
    format: &mut Box<dyn FormatReader>,
) -> Result<(u32, Box<dyn symphonia::core::codecs::Decoder>)> {
    let decoder_opts: DecoderOptions = Default::default();

    // Find the first track that has a decoder available
    for track in format.tracks() {
        if track.codec_params.codec != CODEC_TYPE_NULL
            && let Ok(decoder) =
                symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)
        {
            return Ok((track.id, decoder));
        }
    }

    Err(anyhow::anyhow!("No decodable audio track found"))
}

/// Decode audio packets and collect samples
fn decode_packets(
    format: &mut Box<dyn FormatReader>,
    track_id: u32,
    mut decoder: Box<dyn symphonia::core::codecs::Decoder>,
) -> Result<(Vec<f32>, u32, Channels)> {
    let mut sample_buffer: Option<SampleBuffer<f32>> = None;
    let mut current_spec: Option<SignalSpec> = None;
    
    // Pre-allocate based on duration hint if available
    let n_frames_hint = format.tracks().iter()
        .find(|t| t.id == track_id)
        .and_then(|t| t.codec_params.n_frames)
        .unwrap_or(0);
    
    let n_channels_hint = format.tracks().iter()
        .find(|t| t.id == track_id)
        .and_then(|t| t.codec_params.channels.map(|c| c.count()))
        .unwrap_or(2);

    let mut samples: Vec<f32> = Vec::with_capacity((n_frames_hint * n_channels_hint) as usize);

    let mut actual_sample_rate = None;
    let mut actual_channels = None;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to read packet: {}", e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Initialize or update the sample rate/channels from the first valid packet
                if actual_sample_rate.is_none() {
                    actual_sample_rate = Some(decoded.spec().rate);
                    actual_channels = Some(decoded.spec().channels);
                }

                // Defensive check: Recreate sample buffer if spec changes or capacity is insufficient
                let spec = decoded.spec();
                let frames = decoded.frames();
                let required_capacity = frames * spec.channels.count();

                let needs_recreate = sample_buffer.as_ref().is_none_or(|_| {
                    (current_spec.as_ref() != Some(spec))
                        || sample_buffer.as_ref().unwrap().capacity() < required_capacity
                });

                if needs_recreate {
                    current_spec = Some(*spec);
                    sample_buffer = Some(SampleBuffer::new(required_capacity as u64, *spec));
                    // Update actual metadata when spec changes to avoid stale values
                    actual_sample_rate = Some(spec.rate);
                    actual_channels = Some(spec.channels);
                }

                if let Some(ref mut buf) = sample_buffer {
                    buf.copy_interleaved_ref(decoded);
                    samples.extend_from_slice(buf.samples());
                }
            }
            Err(SymphoniaError::DecodeError(e)) => {
                tracing::warn!("Decode error (skipping packet): {}", e);
                continue;
            }
            Err(e) => return Err(anyhow::anyhow!("Fatal decode error: {}", e)),
        }
    }

    let sample_rate = actual_sample_rate.ok_or_else(|| anyhow::anyhow!("Unknown sample rate"))?;
    let channels = actual_channels.ok_or_else(|| anyhow::anyhow!("Unknown channel count"))?;

    Ok((samples, sample_rate, channels))
}

/// Convert stereo samples to mono using parallel processing
fn convert_to_mono(samples: Vec<f32>, channels: Channels) -> Vec<f32> {
    let n = channels.count();
    if n == 1 {
        return samples;
    }
    let inv_n = 1.0 / n as f32;
    samples
        .par_chunks_exact(n)
        .map(|chunk| chunk.iter().sum::<f32>() * inv_n)
        .collect()
}

/// Resample to target rate if needed.
/// For downsampling, applies a windowed-sinc anti-aliasing filter during the process.
fn resample_to_target(samples: Vec<f32>, sample_rate: u32, target_sample_rate: u32) -> Vec<f32> {
    if sample_rate == target_sample_rate {
        return samples;
    }

    let ratio = sample_rate as f64 / target_sample_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;

    if target_sample_rate < sample_rate {
        let num_taps = 31usize;
        let center = num_taps / 2;
        let cutoff = (target_sample_rate as f32 / 2.0) * 0.9;
        let f_c = cutoff / sample_rate as f32;

        let taps: Vec<f32> = (0..num_taps)
            .map(|i| {
                let n = i as i32 - center as i32;
                if n == 0 {
                    2.0 * f_c
                } else {
                    let n_pi = n as f32 * std::f32::consts::PI;
                    let sinc = (2.0 * f_c * n_pi).sin() / n_pi;
                    let window = 0.54
                        - 0.46
                            * (2.0 * std::f32::consts::PI * i as f32 / (num_taps - 1) as f32).cos();
                    sinc * window
                }
            })
            .collect();

        let sum: f32 = taps.iter().sum();
        let taps: Vec<f32> = taps.into_iter().map(|t| t / sum).collect();
        let mut padded = vec![0.0f32; samples.len() + num_taps];
        padded[center..center + samples.len()].copy_from_slice(&samples);

        (0..output_len)
            .into_par_iter()
            .map(|i| {
                let center_idx = (i as f64 * ratio).round() as usize;
                taps.iter()
                    .enumerate()
                    .map(|(j, &tap)| padded[center_idx + j] * tap)
                    .sum()
            })
            .collect()
    } else {
        (0..output_len)
            .into_par_iter()
            .map(|i| {
                let src_pos = i as f64 * ratio;
                let lo = src_pos.floor() as usize;
                let hi = (lo + 1).min(samples.len() - 1);
                let frac = (src_pos - lo as f64) as f32;
                samples[lo] * (1.0 - frac) + samples[hi] * frac
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_decode_wav_file() {
        // Embed test WAV file
        let wav_data = include_bytes!("../../tests/fixtures/440hz_1sec_mono.wav").to_vec();

        // Decode it
        let result = decode_audio(wav_data, 8000);
        assert!(result.is_ok(), "Should decode WAV file successfully");

        let (samples, duration) = result.unwrap();
        // Should be resampled to 8000 Hz, so 1 second = 8000 samples
        assert_eq!(samples.len(), 8000, "Should have 8000 samples at 8kHz");
        assert!(
            (duration - 1.0).abs() < 0.01,
            "Duration should be ~1 second"
        );
    }

    #[test]
    fn test_decode_stereo_wav() {
        // Embed stereo test WAV file
        let wav_data = include_bytes!("../../tests/fixtures/440hz_1sec_stereo.wav").to_vec();

        // Decode and verify it converts to mono
        let result = decode_audio(wav_data, 8000);
        assert!(result.is_ok(), "Should decode stereo WAV file successfully");

        let (samples, duration) = result.unwrap();
        assert_eq!(samples.len(), 8000, "Should have 8000 samples at 8kHz");
        assert!(
            (duration - 1.0).abs() < 0.01,
            "Duration should be ~1 second"
        );
    }

    #[test]
    fn test_decode_different_durations() {
        // Test 0.5 second file
        let wav_data = include_bytes!("../../tests/fixtures/880hz_0.5sec_mono.wav").to_vec();
        let result = decode_audio(wav_data, 8000);
        assert!(result.is_ok());

        let (samples, duration) = result.unwrap();
        assert_eq!(
            samples.len(),
            4000,
            "0.5 seconds at 8kHz should be 4000 samples"
        );
        assert!(
            (duration - 0.5).abs() < 0.01,
            "Duration should be ~0.5 seconds"
        );
    }

    #[test]
    fn test_decode_silence() {
        // Test with silence file
        let wav_data = include_bytes!("../../tests/fixtures/silence_0.1sec.wav").to_vec();

        // Should decode successfully
        let result = decode_audio(wav_data, 8000);
        assert!(result.is_ok(), "Should decode silence successfully");

        let (samples, duration) = result.unwrap();
        assert_eq!(
            samples.len(),
            800,
            "0.1 seconds at 8kHz should be 800 samples"
        );
        assert!((duration - 0.1).abs() < 0.01);

        // All samples should be near zero (silence)
        let max_abs = samples.iter().fold(0.0, |max, &val| val.abs().max(max));
        assert!(max_abs < 0.0001, "Silence should produce near-zero samples");
    }

    #[test]
    fn test_convert_to_mono() {
        // Test stereo to mono conversion
        let stereo_samples = vec![
            1.0, 0.8, // Frame 1: left=1.0, right=0.8
            0.5, 0.3, // Frame 2: left=0.5, right=0.3
            -0.2, -0.4, // Frame 3: left=-0.2, right=-0.4
        ];

        let mono = convert_to_mono(
            stereo_samples,
            symphonia::core::audio::Channels::FRONT_LEFT
                | symphonia::core::audio::Channels::FRONT_RIGHT,
        );

        // Should average the channels
        assert_eq!(mono.len(), 3);
        assert!((mono[0] - 0.9).abs() < 0.001); // (1.0 + 0.8)/2 = 0.9
        assert!((mono[1] - 0.4).abs() < 0.001); // (0.5 + 0.3)/2 = 0.4
        assert!((mono[2] - (-0.3)).abs() < 0.001); // (-0.2 + -0.4)/2 = -0.3
    }

    #[test]
    fn test_resample_down() {
        // Test downsampling from 44100 to 8000
        let samples: Vec<f32> = (0..44100)
            .map(|i| {
                let t = i as f32 / 44100.0;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            })
            .collect();

        let resampled = resample_to_target(samples.clone(), 44100, 8000);
        assert_eq!(resampled.len(), 8000);

        // Basic check that it's not just zeros
        let max_val = resampled.iter().fold(0.0, |max, &val| val.abs().max(max));
        assert!(
            max_val > 0.1,
            "Resampled signal should have significant amplitude"
        );
    }

    #[test]
    fn test_resample_up() {
        // Test upsampling from 8000 to 16000
        let samples: Vec<f32> = (0..8000)
            .map(|i| {
                let t = i as f32 / 8000.0;
                (2.0 * std::f32::consts::PI * 220.0 * t).sin()
            })
            .collect();

        let resampled = resample_to_target(samples, 8000, 16000);
        assert_eq!(resampled.len(), 16000);
    }

    #[test]
    fn test_resample_same_rate() {
        // Test that resampling to same rate returns original samples
        let samples: Vec<f32> = vec![0.1, 0.5, -0.3, 0.8, -0.9];
        let resampled = resample_to_target(samples.clone(), 8000, 8000);

        // Should be identical when sample rates are the same
        assert_eq!(resampled.len(), samples.len());
        for (i, (&orig, &resampled)) in samples.iter().zip(resampled.iter()).enumerate() {
            assert!(
                (orig - resampled).abs() < 0.0001,
                "Sample {} should be unchanged",
                i
            );
        }
    }

    #[test]
    fn test_invalid_audio_format() {
        // Test with invalid/empty data
        let invalid_data = vec![0u8; 100]; // Too small to be valid WAV
        let result = decode_audio(invalid_data, 8000);
        assert!(result.is_err(), "Should fail with invalid audio data");
    }
}
