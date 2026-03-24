/// 42-feature extraction per frame following the RNNoise Bark-scale band scheme.

pub const BARK_BANDS: usize = 22;

/// 23 edges defining 22 Bark-scale bands in FFT bin indices (48 kHz, 1024-point FFT).
pub const BARK_BAND_EDGES: [usize; 23] = [
    0, 4, 8, 13, 17, 21, 26, 30, 34, 43, 51, 60, 68, 85, 102, 119, 145, 170, 205, 256, 333, 427, 513,
];

pub struct FeatureExtractor {
    prev_band_energy: [f32; BARK_BANDS],
    frame_count: usize,
}

impl FeatureExtractor {
    pub fn new() -> Self {
        FeatureExtractor {
            prev_band_energy: [0.0; BARK_BANDS],
            frame_count: 0,
        }
    }

    /// Extract 42 features from a power spectrum.
    ///
    /// - features[0..22]:  log10 band energies (scaled by 1/10)
    /// - features[22..42]: temporal derivative of the first 20 bands
    ///
    /// `power_spectrum` must have at least `BARK_BAND_EDGES[22]` (513) elements.
    pub fn extract(&mut self, power_spectrum: &[f32]) -> [f32; 42] {
        let mut band_energy = [0.0f32; BARK_BANDS];
        let mut features = [0.0f32; 42];

        // 1. Compute log band energies
        for band in 0..BARK_BANDS {
            let start = BARK_BAND_EDGES[band];
            let end = BARK_BAND_EDGES[band + 1];
            let mut sum = 0.0f32;
            for bin in start..end {
                if bin < power_spectrum.len() {
                    sum += power_spectrum[bin];
                }
            }
            band_energy[band] = (sum + 1e-10).log10() / 10.0;
        }

        // features[0..22] = band log energies
        features[..BARK_BANDS].copy_from_slice(&band_energy);

        // 2. Temporal derivative (first 20 bands)
        if self.frame_count > 0 {
            for i in 0..20 {
                features[22 + i] = band_energy[i] - self.prev_band_energy[i];
            }
        }
        // On the first frame, features[22..42] remain 0.

        // 3. Update state
        self.prev_band_energy = band_energy;
        self.frame_count += 1;

        features
    }
}

impl Default for FeatureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_count() {
        let mut fe = FeatureExtractor::new();
        let spectrum = vec![1.0f32; 513];
        let features = fe.extract(&spectrum);
        assert_eq!(features.len(), 42);
    }

    #[test]
    fn test_first_frame_derivatives_zero() {
        let mut fe = FeatureExtractor::new();
        let spectrum = vec![1.0f32; 513];
        let features = fe.extract(&spectrum);
        for i in 22..42 {
            assert_eq!(features[i], 0.0);
        }
    }

    #[test]
    fn test_second_frame_has_derivatives() {
        let mut fe = FeatureExtractor::new();
        let spectrum1 = vec![1.0f32; 513];
        let _ = fe.extract(&spectrum1);
        let spectrum2 = vec![2.0f32; 513];
        let features2 = fe.extract(&spectrum2);
        // Derivatives should be nonzero because energy changed
        let any_nonzero = features2[22..42].iter().any(|&v| v != 0.0);
        assert!(any_nonzero);
    }
}
