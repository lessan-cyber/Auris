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
    let mss = create_media_source(data);
    let mut format = probe_format(mss)?;
    let (track_id, decoder) = create_decoder(&mut format)?;

    let (samples, sample_rate, channels) = decode_packets(&mut format, track_id, decoder)?;
    let mono_samples = convert_to_mono(samples, channels);
    let resampled = resample_to_target(mono_samples, sample_rate, target_sample_rate);

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
        if track.codec_params.codec != CODEC_TYPE_NULL {
            if let Ok(decoder) =
                symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)
            {
                return Ok((track.id, decoder));
            }
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
    let mut samples: Vec<f32> = Vec::new();

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

                let needs_recreate = sample_buffer.as_ref().map_or(true, |_| {
                    current_spec.as_ref().map_or(true, |s| s != spec)
                        || sample_buffer.as_ref().unwrap().capacity() < required_capacity
                });

                if needs_recreate {
                    current_spec = Some(spec.clone());
                    sample_buffer = Some(SampleBuffer::new(required_capacity as u64, *spec));
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

    #[test]
    fn test_decode_synthetic() {
        // Create a simple sine wave as test data
        // In real tests, include a small test MP3/WAV in repo
        let sample_rate = 44100;
        let duration_secs = 1;
        let samples: Vec<f32> = (0..sample_rate * duration_secs)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
            })
            .collect();

        // Resample test
        let resampled = resample_to_target(samples, 44100, 8000);
        assert_eq!(resampled.len(), 8000);
    }
}
