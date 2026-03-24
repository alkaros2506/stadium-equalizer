use crate::types::Decibels;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum number of samples per frame.
pub const MAX_FRAME_SIZE: usize = 960;

/// Maximum FFT length supported.
pub const MAX_FFT_SIZE: usize = 2048;

/// Default sample rate (Hz).
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

/// Default frame size (10 ms at 48 kHz).
pub const DEFAULT_FRAME_SIZE: usize = 480;

/// Default FFT size.
pub const DEFAULT_FFT_SIZE: usize = 1024;

/// Default hop size (same as default frame size).
pub const DEFAULT_HOP_SIZE: usize = 480;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Which noise-estimation algorithm to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseEstimatorType {
    /// Speech Presence Probability – Minimum Mean Square Error.
    SppMmse,
    /// Martin's Minimum Statistics.
    MartinMs,
}

impl Default for NoiseEstimatorType {
    fn default() -> Self {
        Self::SppMmse
    }
}

/// Broad classification of the acoustic environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentClass {
    OpenStadium,
    DomeArena,
    SmallVenue,
    BroadcastFeed,
}

/// Estimated crowd density.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrowdDensity {
    Sparse,
    Moderate,
    Dense,
    Roaring,
}

/// Current state of the processing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    Idle,
    Calibrating {
        frames_collected: usize,
        target_frames: usize,
    },
    Processing,
    Bypassed,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::Idle
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Core DSP pipeline configuration.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub sample_rate: u32,
    pub frame_size: usize,
    pub fft_size: usize,
    pub hop_size: usize,
    /// Duration of the calibration phase in milliseconds.
    pub calibration_duration_ms: u32,
    /// Which noise-estimation algorithm to use.
    pub noise_estimator_type: NoiseEstimatorType,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            frame_size: DEFAULT_FRAME_SIZE,
            fft_size: DEFAULT_FFT_SIZE,
            hop_size: DEFAULT_HOP_SIZE,
            calibration_duration_ms: 7000,
            noise_estimator_type: NoiseEstimatorType::default(),
        }
    }
}

/// Spectral-processing preset knobs.
#[derive(Debug, Clone, Copy)]
pub struct ProcessingPreset {
    /// Wiener over-subtraction factor (alpha).
    pub wiener_oversubtraction: f32,
    /// Wiener spectral floor (beta).
    pub wiener_floor: f32,
    /// Smoothing coefficient for noise estimate update.
    pub noise_smoothing_alpha: f32,
    /// VAD energy threshold in dB.
    pub vad_threshold: f32,
    /// Noise-gate attack time in milliseconds.
    pub gate_attack_ms: f32,
    /// Noise-gate release time in milliseconds.
    pub gate_release_ms: f32,
}

impl Default for ProcessingPreset {
    fn default() -> Self {
        Self {
            wiener_oversubtraction: 1.5,
            wiener_floor: 0.02,
            noise_smoothing_alpha: 0.98,
            vad_threshold: -40.0,
            gate_attack_ms: 5.0,
            gate_release_ms: 50.0,
        }
    }
}

/// User-facing mix levels (all in dB, 0.0 = unity).
#[derive(Debug, Clone, Copy)]
pub struct UserMix {
    pub crowd_level: f32,
    pub speaker_level: f32,
    pub music_level: f32,
    pub overall_gain: f32,
}

impl Default for UserMix {
    fn default() -> Self {
        Self {
            crowd_level: 0.0,
            speaker_level: 0.0,
            music_level: 0.0,
            overall_gain: 0.0,
        }
    }
}

/// Result of the calibration phase.
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Detected environment type.
    pub environment: EnvironmentClass,
    /// Per-bin noise floor power profile (linear magnitude squared).
    pub noise_floor_profile: Vec<f32>,
    /// Dominant frequency range as (low_hz, high_hz).
    pub dominant_frequency_range: (f32, f32),
    /// Estimated signal-to-noise ratio in dB.
    pub estimated_snr: Decibels,
    /// Estimated crowd density.
    pub crowd_density: CrowdDensity,
    /// Suggested processing preset based on calibration.
    pub recommended_preset: ProcessingPreset,
}
