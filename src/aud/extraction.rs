use std::{f32::consts::PI};
use rustfft::{FftPlanner, num_complex::Complex};

fn fft_magnitude(frames: Vec<Vec<f32>>, sample_rate: u32) -> Vec<Vec<f32>> {
    if frames.is_empty() {return vec![];}

    let n = frames[0].len();
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);

    let mut magnitudes_all = Vec::with_capacity(frames.len());

    for frame in frames {
        let mut buffer: Vec<Complex<f32>> = frame.iter().map(|&v| Complex{re: v, im: 0.0}).collect();

        fft.process(&mut buffer);

        let mags: Vec<f32> = buffer[..n/2]
            .iter()
            .map(|c| (c.re*c.re + c.im*c.im).sqrt())
            .collect();

        magnitudes_all.push(mags); 
    }

    magnitudes_all
}

fn frame(pcm_buffer: &Vec<f32>) -> Vec<Vec<f32>> {
    let frame_size = 1024;
    let hop_size = 512;

    let mut frames: Vec<Vec<f32>> = Vec::new();

    let mut position = 0;
    while position < pcm_buffer.len() {
        let mut frame = vec![0.0; frame_size];

        let end = (position + frame_size).min(pcm_buffer.len());
        let len = end - position;

        frame[..len].copy_from_slice(&pcm_buffer[position.. end]);
        apply_hann_window(&mut frame);
        frames.push(frame);

        position += hop_size
    }

    frames
}

fn apply_hann_window(frame: &mut Vec<f32>) {
    if frame.is_empty() {return;}

    let window = hann_window(frame.len());

    for (sample, w) in frame.iter_mut().zip(window.iter()) {
        *sample *= *w
    }
}

fn hann_window(size: usize) -> Vec<f32> {
    let n = size as f32;
    (0.. size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * PI * i as f32 / (n - 1.0)).cos())
        }).collect()
}