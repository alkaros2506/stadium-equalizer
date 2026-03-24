use super::band_energy::SourceWeights;

/// User mix settings.
///
/// Each level ranges from -1.0 (full suppression) through 0.0 (natural)
/// to 1.0 (maximum boost).
#[derive(Default)]
pub struct UserMix {
    pub crowd_level: f32,
    pub speaker_level: f32,
    pub music_level: f32,
    pub overall_gain_db: f32,
}

/// Converts a user level (-1.0..=1.0) to a linear gain multiplier.
///
/// - level >= 0: 1.0 + level * 2.0  (boost up to 3x at level=1.0)
/// - level <  0: (1.0 + level).max(0.0)  (suppress down to 0x at level=-1.0)
fn level_to_gain(level: f32) -> f32 {
    if level >= 0.0 {
        1.0 + level * 2.0
    } else {
        (1.0 + level).max(0.0)
    }
}

/// Computes per-bin gain masks from source classification weights
/// and user mix preferences.
pub struct MixController {
    mix: UserMix,
    gain_mask: Vec<f32>,
}

impl MixController {
    /// Create a new controller with the default (natural) mix.
    pub fn new(num_bins: usize) -> Self {
        Self {
            mix: UserMix::default(),
            gain_mask: vec![1.0; num_bins],
        }
    }

    /// Update the user mix settings.
    pub fn set_mix(&mut self, mix: UserMix) {
        self.mix = mix;
    }

    /// Return a reference to the current mix settings.
    pub fn get_mix(&self) -> &UserMix {
        &self.mix
    }

    /// Compute the per-bin gain mask from current source weights and user mix.
    ///
    /// Returns a slice of length `num_bins` with linear gain multipliers.
    pub fn compute_gain_mask(&mut self, source_weights: &SourceWeights) -> &[f32] {
        let crowd_gain = level_to_gain(self.mix.crowd_level);
        let speech_gain = level_to_gain(self.mix.speaker_level);
        let music_gain = level_to_gain(self.mix.music_level);

        // overall_gain_db to linear: 10^(dB / 20)
        let overall_linear = 10.0_f32.powf(self.mix.overall_gain_db / 20.0);

        for (((mask, &cw), &sw), &mw) in self
            .gain_mask
            .iter_mut()
            .zip(source_weights.crowd.iter())
            .zip(source_weights.speech.iter())
            .zip(source_weights.music.iter())
        {
            let g = cw * crowd_gain + sw * speech_gain + mw * music_gain;
            *mask = g * overall_linear;
        }

        &self.gain_mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_to_gain_natural() {
        let g = level_to_gain(0.0);
        assert!((g - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_level_to_gain_boost() {
        let g = level_to_gain(1.0);
        assert!((g - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_level_to_gain_suppress() {
        let g = level_to_gain(-1.0);
        assert!(g.abs() < 1e-6);
    }

    #[test]
    fn test_level_to_gain_half_suppress() {
        let g = level_to_gain(-0.5);
        assert!((g - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_default_mix_unity_gain() {
        let weights = SourceWeights {
            crowd: vec![0.33; 4],
            speech: vec![0.34; 4],
            music: vec![0.33; 4],
        };
        let mut ctrl = MixController::new(4);
        let mask = ctrl.compute_gain_mask(&weights);

        for &g in mask {
            // With all levels at 0 and 0 dB overall, gain ~= 1.0
            assert!((g - 1.0).abs() < 0.01, "Expected ~1.0 but got {}", g);
        }
    }

    #[test]
    fn test_suppress_crowd() {
        let weights = SourceWeights {
            crowd: vec![1.0],
            speech: vec![0.0],
            music: vec![0.0],
        };
        let mut ctrl = MixController::new(1);
        ctrl.set_mix(UserMix {
            crowd_level: -1.0,
            speaker_level: 0.0,
            music_level: 0.0,
            overall_gain_db: 0.0,
        });

        let mask = ctrl.compute_gain_mask(&weights);
        assert!(
            mask[0].abs() < 1e-6,
            "Fully suppressed crowd should yield 0 gain"
        );
    }

    #[test]
    fn test_overall_gain_db() {
        let weights = SourceWeights {
            crowd: vec![0.0],
            speech: vec![1.0],
            music: vec![0.0],
        };
        let mut ctrl = MixController::new(1);
        ctrl.set_mix(UserMix {
            crowd_level: 0.0,
            speaker_level: 0.0,
            music_level: 0.0,
            overall_gain_db: 20.0, // +20 dB = 10x
        });

        let mask = ctrl.compute_gain_mask(&weights);
        assert!(
            (mask[0] - 10.0).abs() < 0.01,
            "+20 dB should give ~10x gain, got {}",
            mask[0]
        );
    }

    #[test]
    fn test_boost_speech() {
        let weights = SourceWeights {
            crowd: vec![0.5],
            speech: vec![0.5],
            music: vec![0.0],
        };
        let mut ctrl = MixController::new(1);
        ctrl.set_mix(UserMix {
            crowd_level: 0.0,
            speaker_level: 1.0, // 3x boost
            music_level: 0.0,
            overall_gain_db: 0.0,
        });

        let mask = ctrl.compute_gain_mask(&weights);
        // 0.5 * 1.0 + 0.5 * 3.0 = 2.0
        assert!(
            (mask[0] - 2.0).abs() < 1e-6,
            "Expected 2.0, got {}",
            mask[0]
        );
    }
}
