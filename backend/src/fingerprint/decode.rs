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
fn create_decoder(format: &mut Box<dyn FormatReader>) -> Result<(u32, Box<dyn symphonia::core::codecs::Decoder>)> {
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("No audio track found"))?;

    let track_id = track.id;
    let decoder_opts: DecoderOptions = Default::default();
    
    let decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|e| anyhow::anyhow!("Failed to create decoder: {}", e))?;
    
    Ok((track_id, decoder))
}

/// Decode audio packets and collect samples
fn decode_packets(
    format: &mut Box<dyn FormatReader>,
    track_id: u32,
    mut decoder: Box<dyn symphonia::core::codecs::Decoder>,
) -> Result<(Vec<f32>, u32, Channels)> {
    let mut sample_buffer: Option<SampleBuffer<f32>> = None;
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
                if actual_sample_rate.is_none() {
                    actual_sample_rate = Some(decoded.spec().rate);
                    actual_channels = Some(decoded.spec().channels);

                    let spec =
                        SignalSpec::new(decoded.spec().rate, decoded.spec().channels.clone());
                    let duration = decoded.capacity() as u64;
                    sample_buffer = Some(SampleBuffer::new(duration, spec));
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
    if channels.count() > 1 {
        samples
            .par_chunks(channels.count())
            .map(|chunk| chunk.iter().sum::<f32>() / channels.count() as f32)
            .collect()
    } else {
        samples
    }
}

/// Resample to target rate if needed
fn resample_to_target(samples: Vec<f32>, sample_rate: u32, target_sample_rate: u32) -> Vec<f32> {
    if sample_rate != target_sample_rate {
        resample_linear(&samples, sample_rate, target_sample_rate)
    } else {
        samples
    }
}

/// Simple linear resampling (good enough for fingerprinting) optimized with rayon
fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio) as usize;

    (0..output_len)
        .into_par_iter()
        .map(|i| {
            let src_idx = i as f64 * ratio;
            let src_idx_floor = src_idx.floor() as usize;
            let src_idx_ceil = (src_idx_floor + 1).min(input.len() - 1);
            let frac = src_idx - src_idx_floor as f64;

            input[src_idx_floor] * (1.0 - frac as f32) + input[src_idx_ceil] * frac as f32
        })
        .collect()
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
        let resampled = resample_linear(&samples, 44100, 8000);
        assert_eq!(resampled.len(), 8000);
    }
}
