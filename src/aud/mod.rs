use crate::aud::{decode::ingest, extraction::{fft_magnitude, frame}, hashing::{Fingerprint, find_peaks, generate_hashes}};

mod decode;
mod extraction;
pub mod hashing;

pub fn fingerprints(audio_bytes: Vec<u8>) -> Vec<Fingerprint> {
    let pcm_buffer: Vec<f32> = ingest(&audio_bytes);

    let frames: Vec<Vec<f32>> = frame(&pcm_buffer);

    let magnitudes = fft_magnitude(frames);

    let peaks = find_peaks(magnitudes, 0.01);

    generate_hashes(&peaks, 5, 50)
}