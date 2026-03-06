//! Audio fingerprinting pipeline and modules.
//!
//! This module orchestrates the complete fingerprinting process from raw audio bytes to compact hashes.
//! The pipeline consists of three stages:
//!
//! 1. **Decoding** (`decode`): Convert audio files to normalized PCM samples
//! 2. **Feature Extraction** (`extraction`): Compute spectral features via FFT analysis
//! 3. **Hash Generation** (`hashing`): Identify peaks and create compact fingerprints
//!
//! # Pipeline Overview
//!
//! The [`fingerprint_pipeline`] function chains these stages together. For a detailed understanding
//! of each stage, see the respective module documentation.
//!
//! # Performance
//!
//! - Decoding uses parallel processing via Rayon for resampling
//! - FFT computation is parallelized across frames
//! - All heavy lifting is designed for blocking task execution in async contexts

use crate::fingerprint::{
    decode::ingest,
    extraction::{fft_magnitude, frame},
    hashing::{Fingerprint, find_peaks, generate_hashes},
};

mod decode;
mod extraction;
pub mod hashing;

/// Run the end-to-end fingerprint pipeline on raw audio bytes.
///
/// This function orchestrates the complete fingerprinting workflow:
///
/// 1. **Decode & Resample**: Converts audio to mono PCM at 11,025 Hz
/// 2. **Framing**: Splits PCM into overlapping frames with Hann windowing
/// 3. **Spectral Analysis**: Computes FFT magnitude spectrum for each frame
/// 4. **Peak Detection**: Identifies local spectral peaks above a threshold
/// 5. **Hash Generation**: Pairs peaks to create compact 64-bit fingerprints
///
/// # Arguments
///
/// * `audio_bytes` - Raw audio file bytes (supports MP3, WAV, FLAC, AAC)
///
/// # Returns
///
/// A vector of [`Fingerprint`] objects, each containing:
/// - A 64-bit hash encoding paired spectral peaks
/// - The frame index where the anchor peak was detected
///
/// # Example
///
/// ```no_run
/// # use resonate::fingerprint::fingerprint_pipeline;
/// let audio_data = std::fs::read("song.mp3").unwrap();
/// let fingerprints = fingerprint_pipeline(audio_data);
/// println!("Generated {} fingerprints", fingerprints.len());
/// ```
///
/// # Performance Notes
///
/// This function is CPU-intensive and should be called from a blocking task
/// when used in async contexts (e.g., with `tokio::task::spawn_blocking`).
pub fn fingerprint_pipeline(audio_bytes: Vec<u8>) -> Vec<Fingerprint> {
    let pcm_buffer: Vec<f32> = ingest(&audio_bytes);
    let frames: Vec<Vec<f32>> = frame(&pcm_buffer);
    let magnitudes = fft_magnitude(frames);
    let peaks = find_peaks(magnitudes, 0.01);

    generate_hashes(&peaks, 5, 50)
}