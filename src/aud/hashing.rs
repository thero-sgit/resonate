use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Fingerprint {
    pub hash: u64,
    pub frame_index: usize
}


pub fn generate_hashes(peaks: &[(usize, usize)], fan_value: usize, max_time_diff: usize) -> Vec<Fingerprint> {
    let mut fingerprints = Vec::new();

    for (i, &(t1, f1)) in peaks.iter().enumerate() {
        for j in 1..=fan_value {
            if i + j >= peaks.len() {break;}
            let (t2, f2) = peaks[i + j];
            if t2 - t1 > max_time_diff {break;}

            let hash = ((f1 as u64) << 32) | ((f2 as u64) << 16) | ((t2 - t1) as u64);

            fingerprints.push(
                Fingerprint { hash, frame_index: t1 }
            );
        }
    }

    fingerprints
}

pub fn find_peaks(spectrogram: Vec<Vec<f32>>, magnitude_threshold: f32) -> Vec<(usize, usize)> {
    let number_of_frames = spectrogram.len();
    let number_of_bins = spectrogram[0].len();

    let mut peaks = Vec::new();

    for t in 1.. number_of_frames-1 {
        for f in 1.. number_of_bins-1 {
            let val = spectrogram[t][f];

            if val < magnitude_threshold {continue;}

            let mut is_peak = true;
            for dt in -1..=1 {
                for df in -1..=1 {
                    if dt == 0 && df == 0 {continue;}

                    if spectrogram[(t as isize + dt) as usize][(f as isize + df) as usize] >= val {
                        is_peak = false;
                        break;
                    } 
                }
                if !is_peak {break;}
            }

            if is_peak {
                peaks.push((t, f));
            }
        }
    }

    peaks
}