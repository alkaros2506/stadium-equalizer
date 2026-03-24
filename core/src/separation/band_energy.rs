/// Source type weights per frequency bin.
/// Each vector has length equal to the number of FFT bins,
/// and values are in the range 0.0..=1.0.
pub struct SourceWeights {
    pub crowd: Vec<f32>,
    pub speech: Vec<f32>,
    pub music: Vec<f32>,
}

impl SourceWeights {
    fn new(num_bins: usize) -> Self {
        Self {
            crowd: vec![0.0; num_bins],
            speech: vec![0.0; num_bins],
            music: vec![0.0; num_bins],
        }
    }
}

/// Classifies energy in each frequency bin as belonging to
/// crowd noise, speech, or music based on spectral features.
pub struct BandEnergyAnalyzer {
    num_bins: usize,
    sample_rate: u32,
    fft_size: usize,
    smoothed_weights: SourceWeights,
    smoothing_alpha: f32,
    prev_power: Vec<f32>,
}

/// Gaussian function: exp(-0.5 * ((x - center) / sigma)^2)
fn gaussian(x: f32, center: f32, sigma: f32) -> f32 {
    let z = (x - center) / sigma;
    (-0.5 * z * z).exp()
}

impl BandEnergyAnalyzer {
    /// Create a new analyzer.
    ///
    /// * `num_bins` - number of frequency bins in the power spectrum
    /// * `sample_rate` - audio sample rate in Hz
    /// * `fft_size` - FFT window size used to produce the spectrum
    pub fn new(num_bins: usize, sample_rate: u32, fft_size: usize) -> Self {
        Self {
            num_bins,
            sample_rate,
            fft_size,
            smoothed_weights: SourceWeights::new(num_bins),
            smoothing_alpha: 0.8,
            prev_power: vec![0.0; num_bins],
        }
    }

    /// Analyze a power spectrum and return temporally-smoothed source weights.
    ///
    /// * `power_spectrum` - per-bin power values (length must equal `num_bins`)
    /// * `spectral_flatness` - overall spectral flatness measure in 0.0..=1.0
    /// * `is_speech` - whether voice activity was detected in this frame
    pub fn analyze(
        &mut self,
        power_spectrum: &[f32],
        spectral_flatness: f32,
        is_speech: bool,
    ) -> &SourceWeights {
        let is_speech_factor: f32 = if is_speech { 1.0 } else { 0.0 };
        let epsilon: f32 = 1e-8;

        for bin in 0..self.num_bins {
            let freq = bin as f32 * self.sample_rate as f32 / self.fft_size as f32;

            // --- Crowd noise: 200-2000 Hz, high spectral flatness ---
            let crowd_raw = if (200.0..=2000.0).contains(&freq) {
                gaussian(freq, 800.0, 600.0) * spectral_flatness
            } else {
                0.0
            };

            // --- Speech: 300-4000 Hz, low spectral flatness, VAD active ---
            let speech_raw = if (300.0..=4000.0).contains(&freq) {
                gaussian(freq, 1500.0, 1200.0) * (1.0 - spectral_flatness) * is_speech_factor
            } else {
                0.0
            };

            // --- Music: 80-8000 Hz, moderate flatness, sustained ---
            // Moderate flatness peaks around 0.5, use a gaussian around 0.5 with sigma 0.3
            let flatness_music_scale = gaussian(spectral_flatness, 0.5, 0.3);
            let music_raw = if (80.0..=8000.0).contains(&freq) {
                gaussian(freq, 1000.0, 3000.0) * flatness_music_scale
            } else {
                0.0
            };

            // --- Normalize so crowd + speech + music = 1.0 per bin ---
            let total = crowd_raw + speech_raw + music_raw + epsilon;
            let crowd_norm = crowd_raw / total;
            let speech_norm = speech_raw / total;
            let music_norm = music_raw / total;

            // --- Temporal smoothing ---
            let alpha = self.smoothing_alpha;
            self.smoothed_weights.crowd[bin] =
                alpha * self.smoothed_weights.crowd[bin] + (1.0 - alpha) * crowd_norm;
            self.smoothed_weights.speech[bin] =
                alpha * self.smoothed_weights.speech[bin] + (1.0 - alpha) * speech_norm;
            self.smoothed_weights.music[bin] =
                alpha * self.smoothed_weights.music[bin] + (1.0 - alpha) * music_norm;
        }

        // Store current power for future temporal analysis
        if power_spectrum.len() == self.num_bins {
            self.prev_power.copy_from_slice(power_spectrum);
        }

        &self.smoothed_weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian() {
        let val = gaussian(800.0, 800.0, 600.0);
        assert!((val - 1.0).abs() < 1e-6, "Gaussian at center should be 1.0");

        let off = gaussian(1400.0, 800.0, 600.0);
        assert!(off > 0.0 && off < 1.0);
    }

    #[test]
    fn test_weights_sum_to_one() {
        let mut analyzer = BandEnergyAnalyzer::new(512, 44100, 1024);
        let spectrum = vec![1.0; 512];

        // Run several frames to let smoothing settle
        for _ in 0..50 {
            analyzer.analyze(&spectrum, 0.4, true);
        }
        let weights = analyzer.analyze(&spectrum, 0.4, true);

        for bin in 0..512 {
            let freq = bin as f32 * 44100.0 / 1024.0;
            let sum = weights.crowd[bin] + weights.speech[bin] + weights.music[bin];
            // Bins outside all source ranges (below 80 Hz or above 8000 Hz)
            // have no classification, so their weights remain at zero — skip those.
            if !(80.0..=8000.0).contains(&freq) {
                assert!(
                    sum < 0.01,
                    "Out-of-range bin {} (freq {:.0} Hz) should have near-zero weights, got {}",
                    bin,
                    freq,
                    sum
                );
                continue;
            }
            // After many frames the smoothed values should be very close to normalised
            assert!(
                (sum - 1.0).abs() < 0.05,
                "Weights at bin {} (freq {:.0} Hz) sum to {} instead of ~1.0",
                bin,
                freq,
                sum
            );
        }
    }

    #[test]
    fn test_speech_dominates_midrange_when_vad() {
        let mut analyzer = BandEnergyAnalyzer::new(512, 44100, 1024);
        let spectrum = vec![1.0; 512];

        // Low flatness + speech active should push speech weights up
        for _ in 0..100 {
            analyzer.analyze(&spectrum, 0.05, true);
        }
        let weights = analyzer.analyze(&spectrum, 0.05, true);

        // Bin near 1500 Hz: bin = 1500 * 1024 / 44100 ~ 34
        let bin_1500 = (1500.0 * 1024.0 / 44100.0) as usize;
        assert!(
            weights.speech[bin_1500] > weights.crowd[bin_1500],
            "Speech should dominate at 1500 Hz with low flatness and VAD"
        );
    }

    #[test]
    fn test_crowd_dominates_with_high_flatness() {
        let mut analyzer = BandEnergyAnalyzer::new(512, 44100, 1024);
        let spectrum = vec![1.0; 512];

        for _ in 0..100 {
            analyzer.analyze(&spectrum, 0.9, false);
        }
        let weights = analyzer.analyze(&spectrum, 0.9, false);

        // Bin near 800 Hz
        let bin_800 = (800.0 * 1024.0 / 44100.0) as usize;
        assert!(
            weights.crowd[bin_800] > weights.music[bin_800],
            "Crowd should dominate at 800 Hz with high flatness and no speech"
        );
    }
}
