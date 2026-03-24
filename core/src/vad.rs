/// Voice Activity Detection engine for stadium audio processing.

/// Result of a single VAD analysis frame.
#[derive(Debug, Clone, Copy)]
pub struct VadResult {
    /// Whether speech is detected in this frame.
    pub is_speech: bool,
    /// Smoothed speech probability (0.0 .. 1.0).
    pub probability: f32,
    /// Spectral flatness measure: 0.0 (tonal) .. 1.0 (noise-like).
    pub spectral_flatness: f32,
    /// Frame energy in decibels.
    pub energy_db: f32,
}

/// VAD engine using spectral features to detect speech activity.
pub struct VadEngine {
    energy_threshold_db: f32,
    flatness_threshold: f32,
    smoothed_probability: f32,
    smoothing_alpha: f32,
    sample_rate: u32,
    fft_size: usize,
}

impl VadEngine {
    /// Create a new VAD engine with the given sample rate and FFT size.
    pub fn new(sample_rate: u32, fft_size: usize) -> Self {
        Self {
            energy_threshold_db: -40.0,
            flatness_threshold: 0.3,
            smoothed_probability: 0.0,
            smoothing_alpha: 0.7,
            sample_rate,
            fft_size,
        }
    }

    /// Analyse one frame of power spectrum and return a VAD result.
    ///
    /// `power_spectrum` should contain the magnitude-squared values for each
    /// frequency bin (typically `fft_size / 2 + 1` bins).
    pub fn update(&mut self, power_spectrum: &[f32]) -> VadResult {
        let len = power_spectrum.len();
        if len == 0 {
            return VadResult {
                is_speech: false,
                probability: 0.0,
                spectral_flatness: 0.0,
                energy_db: -100.0,
            };
        }

        let eps: f32 = 1e-10;

        // 1. Frame energy in dB: 10 * log10(mean(power_spectrum))
        let mean_power: f32 = power_spectrum.iter().sum::<f32>() / len as f32;
        let energy_db = 10.0 * (mean_power + eps).log10();

        // 2. Spectral flatness: exp(mean(ln(power_spectrum))) / mean(power_spectrum)
        let log_mean: f32 =
            power_spectrum.iter().map(|&x| (x + eps).ln()).sum::<f32>() / len as f32;
        let geometric_mean = log_mean.exp();
        let arithmetic_mean = mean_power + eps;
        let spectral_flatness = (geometric_mean / arithmetic_mean).clamp(0.0, 1.0);

        // 3. Speech-band energy ratio: energy in 300-4000 Hz / total energy
        let bin_low = (300.0 * self.fft_size as f32 / self.sample_rate as f32) as usize;
        let bin_high = (4000.0 * self.fft_size as f32 / self.sample_rate as f32) as usize;
        let bin_low = bin_low.min(len);
        let bin_high = bin_high.min(len);

        let total_energy: f32 = power_spectrum.iter().sum::<f32>() + eps;
        let speech_band_energy: f32 = if bin_low < bin_high {
            power_spectrum[bin_low..bin_high].iter().sum::<f32>()
        } else {
            0.0
        };
        let speech_band_ratio = speech_band_energy / total_energy;

        // 4. Raw probability: weighted combination of three features.
        let energy_score = if energy_db > self.energy_threshold_db {
            1.0_f32
        } else {
            0.0
        };
        let flatness_score = if spectral_flatness < self.flatness_threshold {
            1.0_f32
        } else {
            0.0
        };
        let band_score = if speech_band_ratio > 0.4 { 1.0_f32 } else { 0.0 };

        let raw_probability = 0.3 * energy_score + 0.4 * flatness_score + 0.3 * band_score;

        // 5. Exponential smoothing.
        self.smoothed_probability = self.smoothing_alpha * self.smoothed_probability
            + (1.0 - self.smoothing_alpha) * raw_probability;

        // 6. Decision.
        let is_speech = self.smoothed_probability > 0.5;

        VadResult {
            is_speech,
            probability: self.smoothed_probability,
            spectral_flatness,
            energy_db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_not_speech() {
        let mut vad = VadEngine::new(48_000, 1024);
        // Very low power → silence
        let spectrum = vec![1e-12_f32; 513];
        for _ in 0..10 {
            let result = vad.update(&spectrum);
            assert!(!result.is_speech);
        }
    }

    #[test]
    fn test_empty_spectrum() {
        let mut vad = VadEngine::new(48_000, 1024);
        let result = vad.update(&[]);
        assert!(!result.is_speech);
        assert_eq!(result.probability, 0.0);
    }

    #[test]
    fn test_loud_tonal_is_speech() {
        let mut vad = VadEngine::new(48_000, 1024);
        // Create a spectrum with strong energy in speech band and tonal character
        let mut spectrum = vec![1e-6_f32; 513];
        // Put energy in 300-4000 Hz range (bins ~6 to ~85 at 48kHz/1024)
        for bin in 6..85 {
            spectrum[bin] = 0.1;
        }
        // Make it tonal by having a few dominant peaks
        spectrum[20] = 10.0;
        spectrum[40] = 8.0;

        // Run several frames to let smoothing converge
        let mut result = vad.update(&spectrum);
        for _ in 0..20 {
            result = vad.update(&spectrum);
        }
        assert!(result.is_speech);
        assert!(result.probability > 0.5);
    }
}
