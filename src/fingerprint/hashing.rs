//! Hash generation and peak-finding for compact fingerprint creation.
//!
//! This module implements the final stage of fingerprinting: converting spectral peaks
//! into compact 64-bit hashes. The hashing strategy pairs adjacent peaks in time and
//! frequency domains, encoding their relative positions into a single hash value.
//!
//! # Hashing Strategy
//!
//! Fingerprints are created by finding spectral peaks in each frame and pairing them
//! with subsequent peaks. Each pair encodes:
//! - Frequency of the anchor peak
//! - Frequency of the paired peak
//! - Time difference between peaks
//!
//! This produces sparse, noise-robust hashes suitable for audio matching.

use serde::{Deserialize, Serialize};

/// A compact audio fingerprint consisting of a hash and frame information.
///
/// This structure represents a single fingerprint from the audio stream.
/// Multiple fingerprints are generated per audio file, enabling robust matching.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Fingerprint {
    /// Compact 64-bit hash encoding paired spectral peaks.
    ///
    /// Encodes frequency and time relationships between peaks.
    /// Format: `(freq1 << 32) | (freq2 << 16) | time_delta`
    pub hash: u64,
    
    /// Index of the frame containing the anchor peak.
    ///
    /// Enables time-localized matching and alignment of fingerprints.
    pub frame_index: usize
}

/// Generate fingerprint hashes from spectral peaks.
///
/// Creates compact hashes by pairing peaks in the spectrogram. For each peak,
/// pairs it with up to `fan_value` subsequent peaks within a time window.
///
/// # Arguments
///
/// * `peaks` - Vector of `(frame_index, frequency_bin)` tuples representing spectral peaks
/// * `fan_value` - Number of consecutive peaks to pair with each anchor peak (typically 5)
/// * `max_time_diff` - Maximum frame distance between paired peaks (typically 50)
///
/// # Returns
///
/// Vector of `Fingerprint` objects, each containing a 64-bit hash and frame index.
///
/// # Algorithm
///
/// For each peak at position `i`:
/// 1. Pair it with peaks `i+1` through `i+fan_value`
/// 2. Only pair peaks where time difference ≤ `max_time_diff`
/// 3. Encode the pair as: `(freq1 << 32) | (freq2 << 16) | time_delta`
///
/// # Example
///
/// ```no_run
/// # use resonate::fingerprint::hashing::generate_hashes;
/// let peaks = vec![(0, 100), (1, 120), (2, 110), (5, 140)];
/// let fingerprints = generate_hashes(&peaks, 5, 50);
/// // Multiple hashes generated from peak pairings
/// assert!(!fingerprints.is_empty());
/// ```
///
/// # Robustness
///
/// This approach creates multiple hashes per location, providing robustness against:
/// - Small pitch shifts or time stretching
/// - Audio degradation or compression artifacts
/// - Partial matches (matching segments of longer songs)
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

/// Detect local spectral peaks in a spectrogram.
///
/// Identifies points in the spectrogram that are local maxima in their 3×3 neighborhood
/// and exceed a magnitude threshold. This creates a sparse representation suitable for hashing.
///
/// # Arguments
///
/// * `spectrogram` - 2D vector: `spectrogram[frame][freq_bin]`
/// * `magnitude_threshold` - Minimum magnitude to consider as a peak
///
/// # Returns
///
/// Vector of `(frame_index, frequency_bin)` tuples for detected peaks.
/// Empty vector if spectrogram is empty.
///
/// # Peak Definition
///
/// A point is a peak if:
/// 1. Its magnitude is ≥ `magnitude_threshold`
/// 2. Its magnitude is strictly greater than all 8 neighbors in time and frequency
/// 3. It is not at the boundary (frame 0 or last, frequency 0 or max)
///
/// # Example
///
/// ```no_run
/// # use resonate::fingerprint::hashing::find_peaks;
/// let spectrogram = vec![
///     vec![0.01, 0.02, 0.01],
///     vec![0.02, 0.5, 0.02],  // 0.5 is peak
///     vec![0.01, 0.02, 0.01],
/// ];
/// let peaks = find_peaks(spectrogram, 0.1);
/// assert_eq!(peaks.len(), 1);
/// assert_eq!(peaks[0], (1, 1));
/// ```
///
/// # Performance
///
/// Scales with spectrogram size. The threshold parameter affects peak density:
/// - Higher threshold → fewer, stronger peaks → sparse fingerprints
/// - Lower threshold → more peaks → denser fingerprints
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