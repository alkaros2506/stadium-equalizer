use super::NoiseEstimator;

/// Martin Minimum Statistics noise PSD estimator based on Martin (2001).
///
/// Tracks the minimum of a smoothed power spectrum across a sliding window
/// of D frames, organized as U subwindows of V frames each. Used as a
/// fallback estimator for the stadium audio equalizer.
pub struct MartinMsEstimator {
    num_bins: usize,
    noise_psd: Vec<f32>,
    smoothed_psd: Vec<f32>,
    /// Ring of U subwindows, each storing the per-bin minimum observed
    /// during that subwindow's V frames.
    subwindow_mins: Vec<Vec<f32>>,
    /// Running minimum for the current (incomplete) subwindow.
    current_subwindow_min: Vec<f32>,
    /// Frame counter within the current subwindow (0..V-1).
    frame_counter: usize,
    /// Index into the subwindow ring for the next completed subwindow.
    subwindow_counter: usize,
    /// U = 8 subwindows.
    num_subwindows: usize,
    /// V = 12 frames per subwindow.
    subwindow_length: usize,
    /// Smoothing coefficient for the input power spectrum.
    alpha_d: f32,
    /// Bias compensation factor Bmin (≈ 1.66 for D=96).
    bias_compensation: f32,
    initialized: bool,
}

/// Floor for PSD values to maintain numerical stability.
const PSD_FLOOR: f32 = 1e-20;

impl MartinMsEstimator {
    /// Create a new Martin Minimum Statistics noise estimator.
    ///
    /// # Arguments
    /// * `num_bins` - Number of frequency bins (typically FFT_SIZE / 2 + 1)
    pub fn new(num_bins: usize) -> Self {
        let num_subwindows: usize = 8; // U
        let subwindow_length: usize = 12; // V = D/U = 96/8

        Self {
            num_bins,
            noise_psd: vec![PSD_FLOOR; num_bins],
            smoothed_psd: vec![PSD_FLOOR; num_bins],
            subwindow_mins: vec![vec![f32::MAX; num_bins]; num_subwindows],
            current_subwindow_min: vec![f32::MAX; num_bins],
            frame_counter: 0,
            subwindow_counter: 0,
            num_subwindows,
            subwindow_length,
            alpha_d: 0.85,
            bias_compensation: 1.66,
            initialized: false,
        }
    }

    /// Compute the overall minimum across all subwindows and the current
    /// running subwindow minimum, then apply bias compensation.
    fn compute_noise_psd(&mut self) {
        for k in 0..self.num_bins {
            let mut overall_min = self.current_subwindow_min[k];
            for sw in &self.subwindow_mins {
                overall_min = overall_min.min(sw[k]);
            }
            self.noise_psd[k] = (self.bias_compensation * overall_min).max(PSD_FLOOR);
        }
    }
}

impl NoiseEstimator for MartinMsEstimator {
    fn update(&mut self, power_spectrum: &[f32]) -> &[f32] {
        let len = power_spectrum.len().min(self.num_bins);

        // First frame: initialize everything from the input spectrum.
        if !self.initialized {
            for k in 0..len {
                let val = power_spectrum[k].max(PSD_FLOOR);
                self.smoothed_psd[k] = val;
                self.current_subwindow_min[k] = val;
                for sw in self.subwindow_mins.iter_mut() {
                    sw[k] = val;
                }
                self.noise_psd[k] = self.bias_compensation * val;
            }
            self.frame_counter = 1;
            self.initialized = true;
            return &self.noise_psd;
        }

        // Step 1: Smooth the input power spectrum.
        let alpha = self.alpha_d;
        for (k, &ps) in power_spectrum.iter().enumerate().take(len) {
            self.smoothed_psd[k] = alpha * self.smoothed_psd[k] + (1.0 - alpha) * ps;
            self.smoothed_psd[k] = self.smoothed_psd[k].max(PSD_FLOOR);
        }

        // Step 2: Update the running minimum for the current subwindow.
        for k in 0..len {
            self.current_subwindow_min[k] = self.current_subwindow_min[k].min(self.smoothed_psd[k]);
        }

        self.frame_counter += 1;

        // Step 3: When we've accumulated V frames, rotate the subwindow ring.
        if self.frame_counter >= self.subwindow_length {
            // Store completed subwindow minimum into the ring.
            let idx = self.subwindow_counter % self.num_subwindows;
            for k in 0..self.num_bins {
                self.subwindow_mins[idx][k] = self.current_subwindow_min[k];
            }

            // Advance the ring pointer.
            self.subwindow_counter = (self.subwindow_counter + 1) % self.num_subwindows;

            // Reset current subwindow tracking.
            self.current_subwindow_min.fill(f32::MAX);
            // Pre-seed with current smoothed values so the new subwindow
            // starts with at least one observation.
            for k in 0..self.num_bins {
                self.current_subwindow_min[k] = self.smoothed_psd[k];
            }

            self.frame_counter = 0;
        }

        // Step 4: Compute noise PSD = Bmin * min across all subwindows.
        self.compute_noise_psd();

        &self.noise_psd
    }

    fn reset(&mut self) {
        self.noise_psd.fill(PSD_FLOOR);
        self.smoothed_psd.fill(PSD_FLOOR);
        for sw in self.subwindow_mins.iter_mut() {
            sw.fill(f32::MAX);
        }
        self.current_subwindow_min.fill(f32::MAX);
        self.frame_counter = 0;
        self.subwindow_counter = 0;
        self.initialized = false;
    }

    fn noise_floor_db(&self) -> Vec<f32> {
        self.noise_psd
            .iter()
            .map(|&psd| 10.0 * psd.max(PSD_FLOOR).log10())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let mut est = MartinMsEstimator::new(4);
        let spectrum = [0.1_f32, 0.2, 0.3, 0.4];
        let result = est.update(&spectrum);
        // First frame: noise_psd = bias_compensation * spectrum
        for (i, &val) in result.iter().enumerate() {
            let expected = 1.66 * spectrum[i];
            assert!(
                (val - expected).abs() < 1e-5,
                "bin {i}: {val} != {expected}"
            );
        }
    }

    #[test]
    fn test_minimum_tracking() {
        let num_bins = 4;
        let mut est = MartinMsEstimator::new(num_bins);

        // Initialize with a high level.
        let high = vec![1.0_f32; num_bins];
        est.update(&high);

        // Feed a lower level for many frames; minimum should track down.
        let low = vec![0.01_f32; num_bins];
        for _ in 0..200 {
            est.update(&low);
        }

        let result = est.update(&low);
        for (k, &val) in result.iter().enumerate() {
            // After convergence the smoothed PSD approaches 0.01,
            // noise_psd ≈ 1.66 * 0.01 = 0.0166
            assert!(
                val < 0.05,
                "bin {k}: noise_psd {val} should have tracked down"
            );
        }
    }

    #[test]
    fn test_reset() {
        let mut est = MartinMsEstimator::new(4);
        est.update(&[1.0, 2.0, 3.0, 4.0]);
        est.reset();
        assert!(!est.initialized);
        assert_eq!(est.frame_counter, 0);
        assert_eq!(est.subwindow_counter, 0);
    }

    #[test]
    fn test_noise_floor_db() {
        let mut est = MartinMsEstimator::new(2);
        est.update(&[1.0, 0.01]);
        let db = est.noise_floor_db();
        // noise_psd = 1.66 * val -> dB = 10*log10(1.66*val)
        let expected_0 = 10.0 * (1.66_f32).log10(); // ~2.2 dB
        let expected_1 = 10.0 * (1.66_f32 * 0.01).log10(); // ~-17.8 dB
        assert!((db[0] - expected_0).abs() < 0.2);
        assert!((db[1] - expected_1).abs() < 0.2);
    }

    #[test]
    fn test_subwindow_rotation() {
        let num_bins = 2;
        let mut est = MartinMsEstimator::new(num_bins);
        let spectrum = vec![0.5_f32; num_bins];

        // Initialize
        est.update(&spectrum);

        // Feed exactly V-1 more frames to complete one subwindow
        // (frame_counter starts at 1 after init, needs to reach subwindow_length=12)
        for _ in 0..11 {
            est.update(&spectrum);
        }

        // After 12 frames total, subwindow should have rotated
        // frame_counter should be reset to 0
        assert_eq!(est.frame_counter, 0);
    }
}
