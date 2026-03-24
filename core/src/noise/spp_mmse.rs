use super::NoiseEstimator;

/// SPP-MMSE Noise PSD Estimator based on Gerkmann & Hendriks (2012).
///
/// Uses speech presence probability to adaptively estimate the noise power
/// spectral density. This is the primary noise estimator for the stadium
/// audio equalizer.
pub struct SppMmseEstimator {
    num_bins: usize,
    noise_psd: Vec<f32>,
    p_smooth: Vec<f32>,
    xi_h1_linear: f32,
    q: f32,
    alpha_noise: f32,
    alpha_p: f32,
    p_threshold: f32,
    initialized: bool,
}

/// Minimum value for gamma to avoid division by zero and log of zero.
const GAMMA_MIN: f32 = 1e-10;

/// Maximum exponent argument to prevent overflow in f32::exp.
const EXP_ARG_MAX: f32 = 50.0;

/// Floor for noise PSD bins to maintain numerical stability.
const NOISE_PSD_FLOOR: f32 = 1e-20;

impl SppMmseEstimator {
    /// Create a new SPP-MMSE noise estimator.
    ///
    /// # Arguments
    /// * `num_bins` - Number of frequency bins (typically FFT_SIZE / 2 + 1)
    pub fn new(num_bins: usize) -> Self {
        // xi_h1 = 15 dB -> linear = 10^(15/10) ≈ 31.62
        let xi_h1_linear: f32 = 10.0_f32.powf(15.0 / 10.0);

        Self {
            num_bins,
            noise_psd: vec![NOISE_PSD_FLOOR; num_bins],
            p_smooth: vec![0.0; num_bins],
            xi_h1_linear,
            q: 0.5,
            alpha_noise: 0.98,
            alpha_p: 0.2,
            p_threshold: 0.99,
            initialized: false,
        }
    }
}

impl NoiseEstimator for SppMmseEstimator {
    fn update(&mut self, power_spectrum: &[f32]) -> &[f32] {
        let len = power_spectrum.len().min(self.num_bins);

        // First frame: initialize noise PSD directly from the input.
        if !self.initialized {
            for (k, &ps) in power_spectrum.iter().enumerate().take(len) {
                self.noise_psd[k] = ps.max(NOISE_PSD_FLOOR);
            }
            self.initialized = true;
            return &self.noise_psd;
        }

        let xi = self.xi_h1_linear;
        let one_plus_xi = 1.0 + xi;
        let xi_ratio = xi / one_plus_xi; // xi_h1 / (1 + xi_h1)
        let inv_one_plus_xi = 1.0 / one_plus_xi; // 1 / (1 + xi_h1)
        let q = self.q;
        let one_minus_q = 1.0 - q;
        let alpha_p = self.alpha_p;
        let alpha_noise = self.alpha_noise;
        let p_threshold = self.p_threshold;

        for (k, &ps) in power_spectrum.iter().enumerate().take(len) {
            // Step 1: a posteriori SNR
            let gamma = (ps / self.noise_psd[k].max(NOISE_PSD_FLOOR)).max(GAMMA_MIN);

            // Step 2: v[k] = (xi_h1 / (1 + xi_h1)) * gamma[k]
            let v = (xi_ratio * gamma).min(EXP_ARG_MAX);

            // Step 3: likelihood ratio lambda[k] = (1/(1+xi_h1)) * exp(v[k])
            let lambda = inv_one_plus_xi * v.exp();

            // Step 4: speech presence probability
            // p[k] = lambda[k]*(1-q) / (lambda[k]*(1-q) + q)
            let numerator = lambda * one_minus_q;
            let denominator = numerator + q;
            let p = if denominator > 0.0 {
                (numerator / denominator).clamp(0.0, 1.0)
            } else {
                0.0
            };

            // Step 5: smooth SPP
            self.p_smooth[k] = alpha_p * self.p_smooth[k] + (1.0 - alpha_p) * p;

            // Step 6: clamp to p_threshold
            self.p_smooth[k] = self.p_smooth[k].min(p_threshold);

            // Step 7: update noise PSD
            self.noise_psd[k] = alpha_noise * self.noise_psd[k]
                + (1.0 - alpha_noise) * (1.0 - self.p_smooth[k]) * power_spectrum[k];

            // Floor the noise PSD to maintain stability
            self.noise_psd[k] = self.noise_psd[k].max(NOISE_PSD_FLOOR);
        }

        &self.noise_psd
    }

    fn reset(&mut self) {
        self.noise_psd.fill(NOISE_PSD_FLOOR);
        self.p_smooth.fill(0.0);
        self.initialized = false;
    }

    fn noise_floor_db(&self) -> Vec<f32> {
        self.noise_psd
            .iter()
            .map(|&psd| 10.0 * psd.max(NOISE_PSD_FLOOR).log10())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let mut est = SppMmseEstimator::new(4);
        let spectrum = [0.1_f32, 0.2, 0.3, 0.4];
        let result = est.update(&spectrum);
        // First frame should copy spectrum into noise PSD
        for (i, &val) in result.iter().enumerate() {
            assert!(
                (val - spectrum[i]).abs() < 1e-6,
                "bin {i}: {val} != {}",
                spectrum[i]
            );
        }
    }

    #[test]
    fn test_noise_only_convergence() {
        let num_bins = 8;
        let mut est = SppMmseEstimator::new(num_bins);
        let noise_level = 0.05_f32;
        let spectrum = vec![noise_level; num_bins];

        // Feed constant noise for many frames; estimate should stay near noise level
        for _ in 0..200 {
            est.update(&spectrum);
        }

        let result = est.update(&spectrum);
        for (k, &val) in result.iter().enumerate() {
            assert!(
                (val - noise_level).abs() < 0.02,
                "bin {k}: noise_psd {val} too far from {noise_level}"
            );
        }
    }

    #[test]
    fn test_reset() {
        let mut est = SppMmseEstimator::new(4);
        est.update(&[1.0, 2.0, 3.0, 4.0]);
        est.reset();
        assert!(!est.initialized);
        assert!(est.p_smooth.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_noise_floor_db() {
        let mut est = SppMmseEstimator::new(2);
        est.update(&[1.0, 0.01]);
        let db = est.noise_floor_db();
        // 10*log10(1.0) = 0 dB, 10*log10(0.01) = -20 dB
        assert!((db[0] - 0.0).abs() < 0.1);
        assert!((db[1] - (-20.0)).abs() < 0.1);
    }
}
