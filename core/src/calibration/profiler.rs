/// Noise-floor profiler that accumulates power spectra during the
/// calibration phase and produces an averaged noise-floor profile.
pub struct NoiseProfiler {
    num_bins: usize,
    accumulated: Vec<f64>,
    frame_count: usize,
    /// Running sum of per-frame energy in dB for averaging.
    energy_db_sum: f64,
    /// Running sum of per-frame spectral flatness for averaging.
    flatness_sum: f64,
}

impl NoiseProfiler {
    /// Create a new profiler for the given number of frequency bins.
    pub fn new(num_bins: usize) -> Self {
        Self {
            num_bins,
            accumulated: vec![0.0; num_bins],
            frame_count: 0,
            energy_db_sum: 0.0,
            flatness_sum: 0.0,
        }
    }

    /// Feed a power spectrum frame into the profiler.
    ///
    /// * `power_spectrum` - Per-bin |X[k]|^2 values.
    pub fn add_frame(&mut self, power_spectrum: &[f32]) {
        let len = power_spectrum.len().min(self.num_bins);
        for (k, &val) in power_spectrum.iter().enumerate().take(len) {
            self.accumulated[k] += val as f64;
        }

        // Compute frame energy in dB.
        let eps: f64 = 1e-20;
        let mean_power: f64 = power_spectrum.iter().map(|&v| v as f64).sum::<f64>()
            / power_spectrum.len().max(1) as f64;
        let energy_db = 10.0 * (mean_power + eps).log10();
        self.energy_db_sum += energy_db;

        // Compute spectral flatness.
        let log_mean: f64 = power_spectrum
            .iter()
            .map(|&v| (v as f64 + eps).ln())
            .sum::<f64>()
            / power_spectrum.len().max(1) as f64;
        let geometric_mean = log_mean.exp();
        let arithmetic_mean = mean_power + eps;
        let flatness = (geometric_mean / arithmetic_mean).clamp(0.0, 1.0);
        self.flatness_sum += flatness;

        self.frame_count += 1;
    }

    /// Return the averaged noise-floor profile (linear power per bin).
    ///
    /// Returns `None` if no frames have been accumulated.
    pub fn noise_floor_profile(&self) -> Option<Vec<f32>> {
        if self.frame_count == 0 {
            return None;
        }
        let inv = 1.0 / self.frame_count as f64;
        Some(self.accumulated.iter().map(|&v| (v * inv) as f32).collect())
    }

    /// Average energy in dB across all accumulated frames.
    pub fn avg_energy_db(&self) -> f32 {
        if self.frame_count == 0 {
            return -100.0;
        }
        (self.energy_db_sum / self.frame_count as f64) as f32
    }

    /// Average spectral flatness across all accumulated frames.
    pub fn avg_spectral_flatness(&self) -> f32 {
        if self.frame_count == 0 {
            return 0.0;
        }
        (self.flatness_sum / self.frame_count as f64) as f32
    }

    /// Number of frames accumulated so far.
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Reset the profiler.
    pub fn reset(&mut self) {
        self.accumulated.fill(0.0);
        self.frame_count = 0;
        self.energy_db_sum = 0.0;
        self.flatness_sum = 0.0;
    }

    /// Find the dominant frequency range (low_hz, high_hz) from the noise
    /// floor, defined as the contiguous band containing 80% of the total
    /// energy.
    pub fn dominant_frequency_range(&self, sample_rate: u32, fft_size: usize) -> (f32, f32) {
        let profile = match self.noise_floor_profile() {
            Some(p) => p,
            None => return (0.0, 0.0),
        };

        let total: f32 = profile.iter().sum();
        if total <= 0.0 {
            return (0.0, 0.0);
        }

        let threshold = total * 0.1; // 10% tails on each side
        let bin_hz = sample_rate as f32 / fft_size as f32;

        // Find low edge.
        let mut cumulative = 0.0_f32;
        let low_bin = profile
            .iter()
            .enumerate()
            .find_map(|(k, &v)| {
                cumulative += v;
                if cumulative >= threshold {
                    Some(k)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        // Find high edge.
        cumulative = 0.0;
        let high_bin = profile
            .iter()
            .enumerate()
            .rev()
            .find_map(|(k, &v)| {
                cumulative += v;
                if cumulative >= threshold {
                    Some(k)
                } else {
                    None
                }
            })
            .unwrap_or(profile.len().saturating_sub(1));

        let low_hz = low_bin as f32 * bin_hz;
        let high_hz = high_bin as f32 * bin_hz;
        (low_hz, high_hz)
    }

    /// Estimate the signal-to-noise ratio in dB.
    ///
    /// Uses the ratio of peak bin power to the median bin power as a rough
    /// SNR proxy.
    pub fn estimated_snr_db(&self) -> f32 {
        let profile = match self.noise_floor_profile() {
            Some(p) => p,
            None => return 0.0,
        };

        let peak = profile.iter().cloned().fold(0.0_f32, f32::max);
        let mut sorted = profile.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[sorted.len() / 2];

        let eps = 1e-20_f32;
        10.0 * ((peak + eps) / (median + eps)).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frames_returns_none() {
        let profiler = NoiseProfiler::new(4);
        assert!(profiler.noise_floor_profile().is_none());
    }

    #[test]
    fn test_single_frame_profile() {
        let mut profiler = NoiseProfiler::new(4);
        profiler.add_frame(&[1.0, 2.0, 3.0, 4.0]);
        let profile = profiler.noise_floor_profile().unwrap();
        assert!((profile[0] - 1.0).abs() < 1e-5);
        assert!((profile[3] - 4.0).abs() < 1e-5);
    }

    #[test]
    fn test_averaging() {
        let mut profiler = NoiseProfiler::new(2);
        profiler.add_frame(&[1.0, 3.0]);
        profiler.add_frame(&[3.0, 1.0]);
        let profile = profiler.noise_floor_profile().unwrap();
        assert!((profile[0] - 2.0).abs() < 1e-5);
        assert!((profile[1] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_reset() {
        let mut profiler = NoiseProfiler::new(2);
        profiler.add_frame(&[1.0, 2.0]);
        profiler.reset();
        assert_eq!(profiler.frame_count(), 0);
        assert!(profiler.noise_floor_profile().is_none());
    }

    #[test]
    fn test_dominant_frequency_range() {
        let mut profiler = NoiseProfiler::new(513);
        // Uniform spectrum
        profiler.add_frame(&vec![1.0; 513]);
        let (low, high) = profiler.dominant_frequency_range(48000, 1024);
        assert!(low < high);
        assert!(low >= 0.0);
    }
}
