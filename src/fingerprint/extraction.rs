//! Spectral feature extraction via FFT analysis.
//!
//! This module provides tools for transforming PCM audio into spectral representations
//! through framing and FFT computation. Key features:
//!
//! - Splits PCM into overlapping frames for time-localized analysis
//! - Applies Hann windowing to reduce spectral leakage
//! - Computes FFT magnitude spectra for each frame
//! - Uses parallel processing for performance
//!
//! # Parameters
//!
//! - **Frame Size**: 1024 samples (at 11,025 Hz ≈ 92.8 ms per frame)
//! - **Hop Size**: 512 samples (50% overlap)
//! - **Window**: Hann window to minimize spectral leakage

use std::f32::consts::PI;
use rustfft::{FftPlanner, num_complex::Complex};
use rayon::prelude::*;

/// Compute FFT magnitude spectra for each audio frame.
///
/// Transforms a vector of windowed frames into their FFT magnitude spectra,
/// keeping only the positive frequency bins (DC to Nyquist).
///
/// # Arguments
///
/// * `frames` - Vector of audio frames, each of length n
///
/// # Returns
///
/// Vector of magnitude spectra, each containing n/2 frequency bins.
/// Empty vector if input is empty.
///
/// # Example
///
/// ```no_run
/// # use resonate::fingerprint::extraction::fft_magnitude;
/// let frames = vec![
///     vec![0.0; 1024],
///     vec![0.1; 1024],
/// ];
/// let magnitudes = fft_magnitude(frames);
/// assert_eq!(magnitudes.len(), 2);
/// assert_eq!(magnitudes[0].len(), 512); // 1024/2
/// ```
///
/// # Performance
///
/// Uses parallel processing via Rayon across frames. FFT computation
/// is done per-frame using rustfft's FftPlanner.
pub fn fft_magnitude(frames: Vec<Vec<f32>>) -> Vec<Vec<f32>> {
    if frames.is_empty() {return vec![];}

    let n = frames[0].len();
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);

    let frames = frames
        .into_par_iter()
        .map(|frame| {
            let mut buffer: Vec<Complex<f32>> = frame.iter().map(|&v| Complex{re: v, im: 0.0}).collect();
            fft.process(&mut buffer);
            
            buffer[..n/2]
                .iter()
                .map(|c| (c.re*c.re + c.im*c.im).sqrt())
                .collect()
            
        })
        .collect();

    frames
}

/// Split PCM audio into overlapping frames with Hann windowing.
///
/// Divides a PCM buffer into frames with 50% overlap (hop size = frame size / 2)
/// and applies a Hann window to each frame to reduce spectral leakage.
///
/// # Arguments
///
/// * `pcm_buffer` - PCM samples to be framed
///
/// # Returns
///
/// Vector of windowed frames, each 1024 samples long. Last frame is zero-padded
/// if the buffer doesn't divide evenly.
///
/// # Parameters
///
/// - **Frame size**: 1024 samples
/// - **Hop size**: 512 samples (50% overlap)
/// - **Window**: Hann window (von Hann)
///
/// # Example
///
/// ```no_run
/// # use resonate::fingerprint::extraction::frame;
/// let pcm = vec![0.1; 8192];
/// let frames = frame(&pcm);
/// // With 50% overlap: (8192 / 512) - 1 ≈ 15 frames
/// assert!(frames.len() >= 15);
/// ```
pub fn frame(pcm_buffer: &Vec<f32>) -> Vec<Vec<f32>> {
    let frame_size = 1024;
    let hop_size = 512;

    let mut frames: Vec<Vec<f32>> = Vec::new();
    let window = hann_window(&frame_size);

    let mut position = 0;
    while position < pcm_buffer.len() {
        let mut frame = vec![0.0; frame_size];

        let end = (position + frame_size).min(pcm_buffer.len());
        let len = end - position;

        frame[..len].copy_from_slice(&pcm_buffer[position.. end]);
        apply_hann_window(&mut frame, &window);
        frames.push(frame);

        position += hop_size
    }

    frames
}

/// Apply a Hann window to an audio frame.
///
/// Multiplies each sample by the corresponding window coefficient
/// element-wise to reduce spectral leakage at frame boundaries.
///
/// # Arguments
///
/// * `frame` - Audio frame to window (modified in-place)
/// * `window` - Pre-computed Hann window coefficients
fn apply_hann_window(frame: &mut Vec<f32>, window: &Vec<f32>) {
    if frame.is_empty() {return;}

    for i in 0..frame.len() {
        frame[i] *= window[i]
    }
}

/// Generate a Hann (von Hann) window.
///
/// Computes the window values for a given size using the formula:
/// $$w[n] = 0.5 \times (1 - \cos(2\pi n / (N-1)))$$
///
/// where $n = 0, 1, ..., N-1$ and $N$ is the window size.
///
/// # Arguments
///
/// * `size` - Number of window coefficients
///
/// # Returns
///
/// Vector of window coefficients (typically in range [0, 1])
fn hann_window(size: &usize) -> Vec<f32> {
    let n = *size as f32;
    (0.. *size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * PI * i as f32 / (n - 1.0)).cos())
        }).collect()
}