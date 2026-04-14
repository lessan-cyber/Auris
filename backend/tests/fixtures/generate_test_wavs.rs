use std::fs::File;
use std::io::Write;

fn generate_wav_file(filename: &str, sample_rate: u32, frequency: f32, duration_secs: f32, channels: u16) {
    let num_samples = (sample_rate as f32 * duration_secs) as u32;
    let bytes_per_sample = 2; // 16-bit
    let data_size = num_samples * bytes_per_sample as u32 * channels as u32;
    let file_size = data_size + 44 - 8; // WAV header is 44 bytes
    
    let mut file = File::create(filename).expect("Failed to create file");
    
    // RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    
    // fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap(); // Subchunk1Size
    file.write_all(&1u16.to_le_bytes()).unwrap();  // AudioFormat (PCM)
    file.write_all(&channels.to_le_bytes()).unwrap(); // NumChannels
    file.write_all(&sample_rate.to_le_bytes()).unwrap(); // SampleRate
    file.write_all(&(sample_rate * bytes_per_sample as u32 * channels as u32).to_le_bytes()).unwrap(); // ByteRate
    file.write_all(&( (bytes_per_sample * channels as u16) as u16 ).to_le_bytes()).unwrap(); // BlockAlign
    file.write_all(&16u16.to_le_bytes()).unwrap(); // BitsPerSample
    
    // data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    
    // Generate samples
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let value = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5;
        let sample = (value * 32767.0) as i16;
        
        for _ in 0..channels {
            file.write_all(&sample.to_le_bytes()).unwrap();
        }
    }
}

fn main() {
    println!("Generating test WAV files...");
    
    // Mono files
    generate_wav_file("440hz_1sec_mono.wav", 44100, 440.0, 1.0, 1);
    generate_wav_file("880hz_0.5sec_mono.wav", 44100, 880.0, 0.5, 1);
    generate_wav_file("220hz_2sec_mono.wav", 44100, 220.0, 2.0, 1);
    
    // Stereo file
    generate_wav_file("440hz_1sec_stereo.wav", 44100, 440.0, 1.0, 2);
    
    // Low sample rate file
    generate_wav_file("440hz_1sec_8k.wav", 8000, 440.0, 1.0, 1);
    
    // Silence
    generate_wav_file("silence_0.1sec.wav", 8000, 0.0, 0.1, 1);
    
    println!("Done! Generated 6 test WAV files.");
}