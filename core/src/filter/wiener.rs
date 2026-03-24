/// Wiener filter gain computation for spectral noise suppression.
///
/// Computes per-bin gains based on estimated SNR:
///   G[k] = max( (|X|^2 - alpha * noise_psd[k]) / |X|^2, beta )
///
/// Where alpha is the oversubtraction factor and beta is the spectral floor.
pub struct WienerFilter {
    alpha: f32,
    beta: f32,
    num_bins: usize,
    gains: Vec<f32>,
}

impl WienerFilter {
    /// Create a new Wiener filter.
    ///
    /// # Arguments
    /// * `num_bins` - Number of frequency bins
    /// * `alpha` - Oversubtraction factor (typical: 1.0..2.0)
    /// * `beta` - Spectral floor, minimum gain (typical: 0.01..0.1)
    pub fn new(num_bins: usize, alpha: f32, beta: f32) -> Self {
        Self {
            alpha,
            beta,
            num_bins,
            gains: vec![1.0; num_bins],
        }
    }

    /// Compute Wiener gains from the power spectrum and noise PSD estimate.
    ///
    /// Both `power_spectrum` and `noise_psd` must have at least `num_bins` elements.
    /// Returns a slice of length `num_bins` containing the computed gains.
    pub fn compute_gains(&mut self, power_spectrum: &[f32], noise_psd: &[f32]) -> &[f32] {
        let n = self.num_bins;
        for ((g, &px), &np) in self
            .gains
            .iter_mut()
            .zip(power_spectrum.iter())
            .zip(noise_psd.iter())
            .take(n)
        {
            if px > 0.0 {
                let gain = (px - self.alpha * np) / px;
                *g = gain.max(self.beta);
            } else {
                // No signal energy; apply spectral floor.
                *g = self.beta;
            }
        }
        &self.gains[..n]
    }

    /// Set the oversubtraction factor.
    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    /// Set the spectral floor (minimum gain).
    pub fn set_beta(&mut self, beta: f32) {
        self.beta = beta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_gain_computation() {
        let mut wf = WienerFilter::new(4, 1.0, 0.02);
        let power = [1.0, 0.5, 0.1, 0.0];
        let noise = [0.2, 0.5, 0.2, 0.0];
        let gains = wf.compute_gains(&power, &noise);
        // bin 0: (1.0 - 0.2)/1.0 = 0.8
        assert!((gains[0] - 0.8).abs() < 1e-6);
        // bin 1: (0.5 - 0.5)/0.5 = 0.0 -> clamped to beta=0.02
        assert!((gains[1] - 0.02).abs() < 1e-6);
        // bin 2: (0.1 - 0.2)/0.1 = -1.0 -> clamped to beta=0.02
        assert!((gains[2] - 0.02).abs() < 1e-6);
        // bin 3: zero power -> beta
        assert!((gains[3] - 0.02).abs() < 1e-6);
    }

    #[test]
    fn test_oversubtraction() {
        let mut wf = WienerFilter::new(2, 2.0, 0.01);
        let power = [1.0, 1.0];
        let noise = [0.3, 0.6];
        let gains = wf.compute_gains(&power, &noise);
        // bin 0: (1.0 - 2.0*0.3)/1.0 = 0.4
        assert!((gains[0] - 0.4).abs() < 1e-6);
        // bin 1: (1.0 - 2.0*0.6)/1.0 = -0.2 -> clamped to 0.01
        assert!((gains[1] - 0.01).abs() < 1e-6);
    }
}
