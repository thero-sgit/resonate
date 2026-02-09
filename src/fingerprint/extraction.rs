//! Spectral extraction helpers.
//!
//! Provides framing, windowing and FFT magnitude computation used by the
//! fingerprinting pipeline.

use std::f32::consts::PI;
use rustfft::{FftPlanner, num_complex::Complex};
use rayon::prelude::*;

/// Compute FFT magnitude spectra for each frame in `frames`.
///
/// Returns a vector of magnitude vectors (only the first `n/2` bins).
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

/// Split a PCM buffer into overlapping frames and apply a Hann window.
///
/// Uses a fixed `frame_size` of 1024 and `hop_size` of 512.
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

fn apply_hann_window(frame: &mut Vec<f32>, window: &Vec<f32>) {
    if frame.is_empty() {return;}

    for i in 0..frame.len() {
        frame[i] *= window[i]
    }
}

fn hann_window(size: &usize) -> Vec<f32> {
    let n = *size as f32;
    (0.. *size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * PI * i as f32 / (n - 1.0)).cos())
        }).collect()
}