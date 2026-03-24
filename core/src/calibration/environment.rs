use crate::config::{CrowdDensity, EnvironmentClass};

/// Features extracted from calibration data for environment classification.
#[derive(Debug, Clone)]
pub struct EnvironmentFeatures {
    /// Average spectral flatness across all calibration frames (0.0..1.0).
    pub avg_spectral_flatness: f32,
    /// Low-frequency energy ratio (below 300 Hz / total).
    pub low_freq_energy_ratio: f32,
    /// Mid-frequency energy ratio (300-4000 Hz / total).
    pub mid_freq_energy_ratio: f32,
    /// High-frequency energy ratio (above 4000 Hz / total).
    pub high_freq_energy_ratio: f32,
    /// Reverberation estimate (ratio of late energy to total energy).
    pub reverb_estimate: f32,
    /// Average frame energy in dB.
    pub avg_energy_db: f32,
}

/// Classify the acoustic environment based on extracted features.
pub fn classify_environment(features: &EnvironmentFeatures) -> EnvironmentClass {
    if features.reverb_estimate < 0.1 && features.avg_spectral_flatness < 0.4 {
        return EnvironmentClass::BroadcastFeed;
    }
    if features.reverb_estimate < 0.3 && features.avg_energy_db < -20.0 {
        return EnvironmentClass::SmallVenue;
    }
    if features.reverb_estimate > 0.4 && features.low_freq_energy_ratio > 0.3 {
        return EnvironmentClass::DomeArena;
    }
    EnvironmentClass::OpenStadium
}

/// Estimate crowd density from spectral and energy features.
pub fn estimate_crowd_density(features: &EnvironmentFeatures) -> CrowdDensity {
    let energy = features.avg_energy_db;
    let flatness = features.avg_spectral_flatness;
    let score = energy + 60.0 + flatness * 30.0;

    if score < 15.0 {
        CrowdDensity::Sparse
    } else if score < 30.0 {
        CrowdDensity::Moderate
    } else if score < 45.0 {
        CrowdDensity::Dense
    } else {
        CrowdDensity::Roaring
    }
}

/// Extract environment features from calibration noise-floor profile.
pub fn extract_features(
    noise_floor: &[f32],
    sample_rate: u32,
    fft_size: usize,
    avg_energy_db: f32,
    avg_spectral_flatness: f32,
) -> EnvironmentFeatures {
    let num_bins = noise_floor.len();
    let bin_hz = sample_rate as f32 / fft_size as f32;
    let eps = 1e-20_f32;

    let total_energy: f32 = noise_floor.iter().sum::<f32>() + eps;

    let bin_300 = (300.0 / bin_hz) as usize;
    let bin_4000 = (4000.0 / bin_hz) as usize;

    let low_energy: f32 = noise_floor[..bin_300.min(num_bins)].iter().sum();
    let mid_energy: f32 = if bin_300 < num_bins && bin_4000 <= num_bins {
        noise_floor[bin_300..bin_4000.min(num_bins)].iter().sum()
    } else {
        0.0
    };
    let high_energy: f32 = if bin_4000 < num_bins {
        noise_floor[bin_4000..].iter().sum()
    } else {
        0.0
    };

    let reverb_bins = (num_bins as f32 * 0.05) as usize;
    let reverb_energy: f32 = noise_floor[..reverb_bins.max(1).min(num_bins)].iter().sum();
    let reverb_estimate = (reverb_energy / total_energy).clamp(0.0, 1.0);

    EnvironmentFeatures {
        avg_spectral_flatness,
        low_freq_energy_ratio: low_energy / total_energy,
        mid_freq_energy_ratio: mid_energy / total_energy,
        high_freq_energy_ratio: high_energy / total_energy,
        reverb_estimate,
        avg_energy_db,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_broadcast() {
        let features = EnvironmentFeatures {
            avg_spectral_flatness: 0.2,
            low_freq_energy_ratio: 0.2,
            mid_freq_energy_ratio: 0.5,
            high_freq_energy_ratio: 0.3,
            reverb_estimate: 0.05,
            avg_energy_db: -30.0,
        };
        assert_eq!(
            classify_environment(&features),
            EnvironmentClass::BroadcastFeed
        );
    }

    #[test]
    fn test_classify_dome() {
        let features = EnvironmentFeatures {
            avg_spectral_flatness: 0.6,
            low_freq_energy_ratio: 0.4,
            mid_freq_energy_ratio: 0.4,
            high_freq_energy_ratio: 0.2,
            reverb_estimate: 0.5,
            avg_energy_db: -10.0,
        };
        assert_eq!(classify_environment(&features), EnvironmentClass::DomeArena);
    }

    #[test]
    fn test_crowd_density_sparse() {
        let features = EnvironmentFeatures {
            avg_spectral_flatness: 0.1,
            low_freq_energy_ratio: 0.2,
            mid_freq_energy_ratio: 0.5,
            high_freq_energy_ratio: 0.3,
            reverb_estimate: 0.1,
            avg_energy_db: -55.0,
        };
        assert_eq!(estimate_crowd_density(&features), CrowdDensity::Sparse);
    }

    #[test]
    fn test_extract_features() {
        let num_bins = 513;
        let noise_floor = vec![0.01_f32; num_bins];
        let features = extract_features(&noise_floor, 48000, 1024, -30.0, 0.5);
        assert!((0.0..=1.0).contains(&features.low_freq_energy_ratio));
        assert!((0.0..=1.0).contains(&features.mid_freq_energy_ratio));
    }
}
