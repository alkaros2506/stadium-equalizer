use crate::calibration::profiler::NoiseProfiler;
use crate::calibration::tuner::CalibrationTuner;
use crate::config::{
    CalibrationResult, NoiseEstimatorType, PipelineConfig, PipelineState, ProcessingPreset,
};
use crate::filter::biquad_bank::BiquadBank;
use crate::filter::spectral_gate::SpectralGate;
use crate::filter::wiener::WienerFilter;
use crate::noise::martin_ms::MartinMsEstimator;
use crate::noise::spp_mmse::SppMmseEstimator;
use crate::noise::NoiseEstimator;
use crate::separation::band_energy::BandEnergyAnalyzer;
use crate::separation::mix_controller::{MixController, UserMix};
use crate::stft::StftEngine;
use crate::vad::VadEngine;

// ---------------------------------------------------------------------------
// Soft-limiter utility
// ---------------------------------------------------------------------------

/// Soft-clip a sample using tanh compression above the threshold.
///
/// For |x| <= threshold the sample passes unmodified.
/// For |x| > threshold: `threshold * tanh(x / threshold)`.
fn soft_clip(x: f32, threshold: f32) -> f32 {
    if threshold <= 0.0 {
        return 0.0;
    }
    if x.abs() <= threshold {
        x
    } else {
        threshold * (x / threshold).tanh()
    }
}

// ---------------------------------------------------------------------------
// Default constants
// ---------------------------------------------------------------------------

/// Default temporal smoothing coefficient for gain interpolation.
const DEFAULT_GAIN_SMOOTHING_ALPHA: f32 = 0.85;

/// Default limiter threshold in linear amplitude.
const DEFAULT_LIMITER_THRESHOLD: f32 = 0.95;

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Top-level audio processing pipeline that orchestrates calibration,
/// noise estimation, spectral filtering, source separation, mixing,
/// time-domain EQ, and output limiting.
pub struct Pipeline {
    config: PipelineConfig,
    state: PipelineState,
    stft: StftEngine,
    noise_estimator: Box<dyn NoiseEstimator>,
    vad: VadEngine,
    wiener: WienerFilter,
    band_analyzer: BandEnergyAnalyzer,
    mix_controller: MixController,
    eq_bank: BiquadBank,
    // Calibration
    profiler: Option<NoiseProfiler>,
    tuner: CalibrationTuner,
    calibration_result: Option<CalibrationResult>,
    // Gain smoothing
    prev_gains: Vec<f32>,
    combined_gains: Vec<f32>,
    gain_smoothing_alpha: f32,
    // Output limiter
    limiter_threshold: f32,
}

impl Pipeline {
    /// Create a new pipeline with the given configuration.
    ///
    /// All internal components are constructed from the config values.
    pub fn new(config: PipelineConfig) -> Self {
        let num_bins = config.fft_size / 2 + 1;

        let noise_estimator: Box<dyn NoiseEstimator> = match config.noise_estimator_type {
            NoiseEstimatorType::SppMmse => Box::new(SppMmseEstimator::new(num_bins)),
            NoiseEstimatorType::MartinMs => Box::new(MartinMsEstimator::new(num_bins)),
        };

        let preset = ProcessingPreset::default();

        Self {
            stft: StftEngine::new(config.fft_size, config.hop_size),
            noise_estimator,
            vad: VadEngine::new(config.sample_rate, config.fft_size),
            wiener: WienerFilter::new(num_bins, preset.wiener_oversubtraction, preset.wiener_floor),
            band_analyzer: BandEnergyAnalyzer::new(num_bins, config.sample_rate, config.fft_size),
            mix_controller: MixController::new(num_bins),
            eq_bank: BiquadBank::default_stadium_eq(config.sample_rate as f32),
            profiler: None,
            tuner: CalibrationTuner::new(config.sample_rate, config.fft_size),
            calibration_result: None,
            prev_gains: vec![1.0; num_bins],
            combined_gains: vec![0.0; num_bins],
            gain_smoothing_alpha: DEFAULT_GAIN_SMOOTHING_ALPHA,
            limiter_threshold: DEFAULT_LIMITER_THRESHOLD,
            state: PipelineState::Idle,
            config,
        }
    }

    // ------------------------------------------------------------------
    // State queries
    // ------------------------------------------------------------------

    /// Return the current pipeline state.
    pub fn get_state(&self) -> &PipelineState {
        &self.state
    }

    /// Return the calibration result, if calibration has been completed.
    pub fn get_calibration_result(&self) -> Option<&CalibrationResult> {
        self.calibration_result.as_ref()
    }

    // ------------------------------------------------------------------
    // State transitions
    // ------------------------------------------------------------------

    /// Begin the calibration phase.
    ///
    /// Allocates a fresh `NoiseProfiler` and transitions the pipeline to the
    /// `Calibrating` state. The number of target frames is derived from
    /// `calibration_duration_ms`, `sample_rate`, and `hop_size`.
    pub fn start_calibration(&mut self) {
        let num_bins = self.config.fft_size / 2 + 1;
        let target_frames = (self.config.calibration_duration_ms as usize
            * self.config.sample_rate as usize)
            / (self.config.hop_size * 1000);

        self.profiler = Some(NoiseProfiler::new(num_bins));
        self.noise_estimator.reset();
        self.state = PipelineState::Calibrating {
            frames_collected: 0,
            target_frames,
        };
    }

    /// Enable or disable bypass mode.
    ///
    /// When bypass is enabled the pipeline passes audio through unmodified.
    /// Disabling bypass returns to `Processing` state.
    pub fn set_bypass(&mut self, bypass: bool) {
        self.state = if bypass {
            PipelineState::Bypassed
        } else {
            PipelineState::Processing
        };
    }

    /// Update the user mix settings forwarded to the `MixController`.
    pub fn set_mix(&mut self, mix: UserMix) {
        self.mix_controller.set_mix(mix);
    }

    /// Apply a processing preset (updates Wiener parameters, etc.).
    pub fn apply_preset(&mut self, preset: &ProcessingPreset) {
        self.wiener.set_alpha(preset.wiener_oversubtraction);
        self.wiener.set_beta(preset.wiener_floor);
    }

    // ------------------------------------------------------------------
    // Main processing entry point
    // ------------------------------------------------------------------

    /// Process a single hop-sized frame of audio.
    ///
    /// The behaviour depends on the current pipeline state:
    ///
    /// * **Idle / Bypassed** -- input is copied to output unchanged.
    /// * **Calibrating** -- spectrum is analysed to feed the noise profiler
    ///   and noise estimator. When enough frames have been collected the
    ///   calibration is finalised and the pipeline transitions to
    ///   `Processing`. Audio passes through unmodified.
    /// * **Processing** -- full signal chain: STFT analysis, noise
    ///   estimation, VAD, band analysis, Wiener + mix gains, temporal
    ///   smoothing, spectral gating, STFT synthesis, time-domain EQ, and
    ///   soft limiting.
    pub fn process_frame(&mut self, input: &[f32], output: &mut Vec<f32>) {
        match self.state {
            PipelineState::Idle | PipelineState::Bypassed => {
                self.copy_input_to_output(input, output);
            }
            PipelineState::Calibrating {
                frames_collected,
                target_frames,
            } => {
                self.process_calibrating(input, output, frames_collected, target_frames);
            }
            PipelineState::Processing => {
                self.process_full_chain(input, output);
            }
        }
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Copy input samples to output, resizing as needed.
    fn copy_input_to_output(&self, input: &[f32], output: &mut Vec<f32>) {
        output.clear();
        output.extend_from_slice(input);
    }

    /// Calibration-phase processing.
    fn process_calibrating(
        &mut self,
        input: &[f32],
        output: &mut Vec<f32>,
        frames_collected: usize,
        target_frames: usize,
    ) {
        // Analyse the frame to obtain the spectrum.
        let spectrum = self.stft.analyze(input);
        let power: Vec<f32> = spectrum.iter().map(|c| c.norm_sqr()).collect();

        // Feed the noise profiler.
        if let Some(ref mut profiler) = self.profiler {
            profiler.add_frame(&power);
        }

        // Also warm up the noise estimator during calibration.
        self.noise_estimator.update(&power);

        let new_collected = frames_collected + 1;

        if new_collected >= target_frames {
            // Finalise calibration.
            self.finalize_calibration();
        } else {
            self.state = PipelineState::Calibrating {
                frames_collected: new_collected,
                target_frames,
            };
        }

        // Pass audio through unmodified during calibration.
        self.copy_input_to_output(input, output);
    }

    /// Finalise the calibration phase: run the tuner, apply the recommended
    /// preset, and transition to `Processing`.
    fn finalize_calibration(&mut self) {
        if let Some(ref profiler) = self.profiler {
            let result = self.tuner.finalize(profiler);
            if let Some(ref cal) = result {
                self.apply_preset(&cal.recommended_preset);
                self.calibration_result = Some(cal.clone());
            }
        }
        self.profiler = None;
        self.state = PipelineState::Processing;
    }

    /// Full processing chain (used in the `Processing` state).
    fn process_full_chain(&mut self, input: &[f32], output: &mut Vec<f32>) {
        let num_bins = self.prev_gains.len();

        // 1. STFT analysis.
        let mut spectrum = self.stft.analyze(input);

        // 2. Power spectrum.
        let power: Vec<f32> = spectrum.iter().map(|c| c.norm_sqr()).collect();

        // 3. Update noise estimator.
        let noise_psd = self.noise_estimator.update(&power).to_vec();

        // 4. Voice activity detection.
        let vad_result = self.vad.update(&power);

        // 5. Band energy / source separation analysis.
        let source_weights = self.band_analyzer.analyze(
            &power,
            vad_result.spectral_flatness,
            vad_result.is_speech,
        );

        // 6. Wiener filter gains.
        let wiener_gains = self.wiener.compute_gains(&power, &noise_psd).to_vec();

        // 7. Mix-controller gains.
        let mix_gains = self.mix_controller.compute_gain_mask(source_weights).to_vec();

        // 8. Combine Wiener and mix gains.
        SpectralGate::combine_gains(&[&wiener_gains, &mix_gains], &mut self.combined_gains);

        // 9. Temporal gain smoothing.
        let mut smoothed = vec![0.0_f32; num_bins];
        SpectralGate::smooth_gains(
            &self.prev_gains,
            &self.combined_gains,
            self.gain_smoothing_alpha,
            &mut smoothed,
        );
        self.prev_gains.copy_from_slice(&smoothed);

        // 10. Apply smoothed gains to the spectrum.
        SpectralGate::apply(&mut spectrum, &smoothed);

        // 11. STFT synthesis.
        let mut samples = self.stft.synthesize(&mut spectrum);

        // 12. Time-domain EQ.
        self.eq_bank.process(&mut samples);

        // 13. Soft limiter.
        let threshold = self.limiter_threshold;
        for s in samples.iter_mut() {
            *s = soft_clip(*s, threshold);
        }

        // Write to output.
        output.clear();
        output.extend_from_slice(&samples);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PipelineConfig;

    #[test]
    fn test_idle_passes_through() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        let input = vec![0.5_f32; 480];
        let mut output = Vec::new();
        pipeline.process_frame(&input, &mut output);
        assert_eq!(output.len(), 480);
        assert_eq!(&output, &input);
    }

    #[test]
    fn test_bypass_passes_through() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        pipeline.set_bypass(true);

        let input: Vec<f32> = (0..480).map(|i| i as f32 * 0.001).collect();
        let mut output = Vec::new();
        pipeline.process_frame(&input, &mut output);
        assert_eq!(output.len(), 480);
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-7);
        }
    }

    #[test]
    fn test_calibration_collects_and_transitions() {
        let config = PipelineConfig {
            calibration_duration_ms: 50,
            ..PipelineConfig::default()
        };
        let mut pipeline = Pipeline::new(config);
        pipeline.start_calibration();

        match pipeline.get_state() {
            PipelineState::Calibrating { target_frames, .. } => {
                assert!(*target_frames > 0);
            }
            _ => panic!("Expected Calibrating state"),
        }

        let input = vec![0.01_f32; 480];
        let mut output = Vec::new();

        // Feed enough frames to finish calibration.
        for _ in 0..500 {
            pipeline.process_frame(&input, &mut output);
        }

        assert_eq!(*pipeline.get_state(), PipelineState::Processing);
    }

    #[test]
    fn test_processing_produces_output() {
        let config = PipelineConfig {
            calibration_duration_ms: 50,
            ..PipelineConfig::default()
        };
        let mut pipeline = Pipeline::new(config);
        pipeline.start_calibration();

        let input = vec![0.01_f32; 480];
        let mut output = Vec::new();

        for _ in 0..500 {
            pipeline.process_frame(&input, &mut output);
        }

        assert_eq!(*pipeline.get_state(), PipelineState::Processing);
        pipeline.process_frame(&input, &mut output);
        assert_eq!(output.len(), 480);
    }

    #[test]
    fn test_set_mix_does_not_panic() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        pipeline.set_mix(UserMix {
            crowd_level: -0.5,
            speaker_level: 0.8,
            music_level: 0.0,
            overall_gain_db: -3.0,
        });
    }

    #[test]
    fn test_apply_preset_does_not_panic() {
        let mut pipeline = Pipeline::new(PipelineConfig::default());
        let preset = ProcessingPreset {
            wiener_oversubtraction: 2.0,
            wiener_floor: 0.05,
            ..ProcessingPreset::default()
        };
        pipeline.apply_preset(&preset);
    }

    #[test]
    fn test_soft_clip_passthrough_below_threshold() {
        assert_eq!(soft_clip(0.5, 1.0), 0.5);
        assert_eq!(soft_clip(-0.3, 1.0), -0.3);
    }

    #[test]
    fn test_soft_clip_compresses_above_threshold() {
        let clipped = soft_clip(2.0, 0.9);
        assert!(clipped < 2.0, "Should compress");
        assert!(clipped > 0.0, "Should remain positive");
        assert!(clipped <= 0.9, "Should not exceed threshold");
    }

    #[test]
    fn test_soft_clip_symmetry() {
        let pos = soft_clip(1.5, 0.8);
        let neg = soft_clip(-1.5, 0.8);
        assert!((pos + neg).abs() < 1e-7, "Soft clip should be odd-symmetric");
    }

    #[test]
    fn test_get_calibration_result_none_before_calibration() {
        let pipeline = Pipeline::new(PipelineConfig::default());
        assert!(pipeline.get_calibration_result().is_none());
    }
}
