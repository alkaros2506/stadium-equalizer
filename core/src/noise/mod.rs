pub mod martin_ms;
pub mod spp_mmse;

/// Trait for noise PSD estimators
pub trait NoiseEstimator {
    /// Update the noise estimate with a new power spectrum frame.
    /// power_spectrum contains |X[k]|^2 for each frequency bin.
    /// Returns the current noise PSD estimate.
    fn update(&mut self, power_spectrum: &[f32]) -> &[f32];

    /// Reset the estimator to initial state
    fn reset(&mut self);

    /// Get the current noise floor estimate in dB
    fn noise_floor_db(&self) -> Vec<f32>;
}
