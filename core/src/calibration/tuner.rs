use crate::config::{
    CalibrationResult, CrowdDensity, EnvironmentClass, ProcessingPreset,
};
use crate::types::Decibels;

use super::environment::{self, EnvironmentFeatures};
use super::profiler::NoiseProfiler;

/// Generates a CalibrationResult from collected profiler data.
pub struct CalibrationTuner {
    sample_rate: u32,
    fft_size: usize,
}

impl CalibrationTuner {
    /// Create a tuner for the given audio parameters.
    pub fn new(sample_rate: u32, fft_size: usize) -> Self {
        Self {
            sample_rate,
            fft_size,
        }
    }

    /// Produce a full calibration result from the profiler's accumulated data.
    ///
    /// Returns `None` if the profiler has no frames.
    pub fn finalize(&self, profiler: &NoiseProfiler) -> Option<CalibrationResult> {
        let noise_floor_profile = profiler.noise_floor_profile()?;

        let avg_energy_db = profiler.avg_energy_db();
        let avg_flatness = profiler.avg_spectral_flatness();

        let features = environment::extract_features(
            &noise_floor_profile,
            self.sample_rate,
            self.fft_size,
            avg_energy_db,
            avg_flatness,
        );

        let env_class = environment::classify_environment(&features);
        let crowd_density = environment::estimate_crowd_density(&features);

        let dominant_frequency_range =
            profiler.dominant_frequency_range(self.sample_rate, self.fft_size);

        let estimated_snr = Decibels(profiler.estimated_snr_db());

        let recommended_preset = recommend_preset(&env_class, &crowd_density, &features);

        Some(CalibrationResult {
            environment: env_class,
            noise_floor_profile,
            dominant_frequency_range,
            estimated_snr,
            crowd_density,
            recommended_preset,
        })
    }
}

/// Choose a processing preset based on environment classification and crowd density.
fn recommend_preset(
    env: &EnvironmentClass,
    crowd: &CrowdDensity,
    features: &EnvironmentFeatures,
) -> ProcessingPreset {
    let mut preset = ProcessingPreset::default();

    // Adjust oversubtraction based on noise level.
    match crowd {
        CrowdDensity::Roaring => {
            preset.wiener_oversubtraction = 2.0;
            preset.wiener_floor = 0.03;
        }
        CrowdDensity::Dense => {
            preset.wiener_oversubtraction = 1.8;
            preset.wiener_floor = 0.025;
        }
        CrowdDensity::Moderate => {
            preset.wiener_oversubtraction = 1.5;
            preset.wiener_floor = 0.02;
        }
        CrowdDensity::Sparse => {
            preset.wiener_oversubtraction = 1.2;
            preset.wiener_floor = 0.015;
        }
    }

    // Adjust smoothing for reverberant environments.
    match env {
        EnvironmentClass::DomeArena => {
            preset.noise_smoothing_alpha = 0.95;
            preset.gate_release_ms = 80.0;
        }
        EnvironmentClass::SmallVenue => {
            preset.noise_smoothing_alpha = 0.96;
            preset.gate_release_ms = 40.0;
        }
        EnvironmentClass::BroadcastFeed => {
            preset.noise_smoothing_alpha = 0.99;
            preset.gate_release_ms = 30.0;
        }
        EnvironmentClass::OpenStadium => {
            // defaults are fine
        }
    }

    // Adjust VAD threshold based on average energy.
    if features.avg_energy_db > -20.0 {
        preset.vad_threshold = -35.0; // louder environment, raise threshold
    } else if features.avg_energy_db < -40.0 {
        preset.vad_threshold = -45.0; // quiet, lower threshold
    }

    preset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finalize_no_frames() {
        let profiler = NoiseProfiler::new(4);
        let tuner = CalibrationTuner::new(48000, 1024);
        assert!(tuner.finalize(&profiler).is_none());
    }

    #[test]
    fn test_finalize_produces_result() {
        let mut profiler = NoiseProfiler::new(513);
        for _ in 0..100 {
            profiler.add_frame(&vec![0.01; 513]);
        }
        let tuner = CalibrationTuner::new(48000, 1024);
        let result = tuner.finalize(&profiler);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.noise_floor_profile.len(), 513);
        assert!(result.dominant_frequency_range.0 < result.dominant_frequency_range.1);
    }

    #[test]
    fn test_recommend_preset_roaring() {
        let features = EnvironmentFeatures {
            avg_spectral_flatness: 0.8,
            low_freq_energy_ratio: 0.3,
            mid_freq_energy_ratio: 0.5,
            high_freq_energy_ratio: 0.2,
            reverb_estimate: 0.2,
            avg_energy_db: -10.0,
        };
        let preset = recommend_preset(
            &EnvironmentClass::OpenStadium,
            &CrowdDensity::Roaring,
            &features,
        );
        assert!((preset.wiener_oversubtraction - 2.0).abs() < 1e-6);
        assert!(preset.vad_threshold > -40.0); // raised for loud environment
    }
}
