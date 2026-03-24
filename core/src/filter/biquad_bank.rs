use std::f32::consts::PI;

/// Second-order (biquad) filter coefficients in normalized Direct Form I.
///
/// Transfer function: H(z) = (b0 + b1*z^-1 + b2*z^-2) / (1 + a1*z^-1 + a2*z^-2)
///
/// All coefficients are pre-normalized (a0 = 1).
#[derive(Debug, Clone)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoeffs {
    /// Design a peaking EQ filter using Audio EQ Cookbook formulas.
    ///
    /// * `sample_rate` - Sample rate in Hz
    /// * `freq` - Center frequency in Hz
    /// * `q` - Quality factor
    /// * `gain_db` - Gain at center frequency in dB (positive = boost, negative = cut)
    pub fn peaking_eq(sample_rate: f32, freq: f32, q: f32, gain_db: f32) -> Self {
        let a_lin = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0 + alpha * a_lin;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a_lin;
        let a0 = 1.0 + alpha / a_lin;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a_lin;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Design a high-pass filter using Audio EQ Cookbook formulas.
    ///
    /// * `sample_rate` - Sample rate in Hz
    /// * `freq` - Cutoff frequency in Hz
    /// * `q` - Quality factor
    pub fn high_pass(sample_rate: f32, freq: f32, q: f32) -> Self {
        let w0 = 2.0 * PI * freq / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Design a low-pass filter using Audio EQ Cookbook formulas.
    ///
    /// * `sample_rate` - Sample rate in Hz
    /// * `freq` - Cutoff frequency in Hz
    /// * `q` - Quality factor
    pub fn low_pass(sample_rate: f32, freq: f32, q: f32) -> Self {
        let w0 = 2.0 * PI * freq / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

/// A single biquad filter with internal state (Direct Form I).
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    coeffs: BiquadCoeffs,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    /// Create a new biquad filter from the given coefficients.
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Process a single sample through the filter (Direct Form I).
    ///
    /// y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let c = &self.coeffs;
        let y = c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;

        y
    }

    /// Reset the filter state to zero.
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// A bank of biquad filters applied in series for parametric EQ shaping.
#[derive(Default)]
pub struct BiquadBank {
    filters: Vec<BiquadFilter>,
}

impl BiquadBank {
    /// Create an empty biquad bank.
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Add a filter to the bank.
    pub fn add_filter(&mut self, filter: BiquadFilter) {
        self.filters.push(filter);
    }

    /// Process a buffer of samples through all filters in series, in-place.
    pub fn process(&mut self, samples: &mut [f32]) {
        for filter in &mut self.filters {
            for sample in samples.iter_mut() {
                *sample = filter.process_sample(*sample);
            }
        }
    }

    /// Reset state of all filters in the bank.
    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
    }

    /// Create a bank with reasonable default EQ settings for stadium audio.
    ///
    /// Includes:
    /// - High-pass at 60 Hz (remove rumble)
    /// - Peaking cut at 250 Hz, Q=1.0, -3 dB (reduce muddiness)
    /// - Peaking boost at 3000 Hz, Q=1.5, +2 dB (speech presence)
    /// - Peaking cut at 8000 Hz, Q=2.0, -2 dB (reduce sibilance/harshness)
    pub fn default_stadium_eq(sample_rate: f32) -> Self {
        let mut bank = Self::new();

        bank.add_filter(BiquadFilter::new(BiquadCoeffs::high_pass(
            sample_rate,
            60.0,
            0.707,
        )));
        bank.add_filter(BiquadFilter::new(BiquadCoeffs::peaking_eq(
            sample_rate,
            250.0,
            1.0,
            -3.0,
        )));
        bank.add_filter(BiquadFilter::new(BiquadCoeffs::peaking_eq(
            sample_rate,
            3000.0,
            1.5,
            2.0,
        )));
        bank.add_filter(BiquadFilter::new(BiquadCoeffs::peaking_eq(
            sample_rate,
            8000.0,
            2.0,
            -2.0,
        )));

        bank
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peaking_eq_unity_at_zero_gain() {
        let c = BiquadCoeffs::peaking_eq(48000.0, 1000.0, 1.0, 0.0);
        // At 0 dB gain, A=1 so numerator == denominator after normalization.
        // All normalized coefficients should match: b0/a0=1, b1=a1, b2=a2.
        assert!((c.b0 - 1.0).abs() < 1e-5);
        assert!((c.b1 - c.a1).abs() < 1e-5);
        assert!((c.b2 - c.a2).abs() < 1e-5);
    }

    #[test]
    fn test_high_pass_blocks_dc() {
        let coeffs = BiquadCoeffs::high_pass(48000.0, 200.0, 0.707);
        let mut filter = BiquadFilter::new(coeffs);

        // Feed DC signal; output should converge toward zero
        let mut last = 0.0_f32;
        for _ in 0..10000 {
            last = filter.process_sample(1.0);
        }
        assert!(last.abs() < 0.01, "HPF should block DC, got {}", last);
    }

    #[test]
    fn test_low_pass_passes_dc() {
        let coeffs = BiquadCoeffs::low_pass(48000.0, 5000.0, 0.707);
        let mut filter = BiquadFilter::new(coeffs);

        // Feed DC signal; output should converge toward 1.0
        let mut last = 0.0_f32;
        for _ in 0..10000 {
            last = filter.process_sample(1.0);
        }
        assert!(
            (last - 1.0).abs() < 0.01,
            "LPF should pass DC, got {}",
            last
        );
    }

    #[test]
    fn test_bank_process() {
        let mut bank = BiquadBank::default_stadium_eq(48000.0);
        let mut samples = vec![0.0_f32; 1024];
        // Impulse
        samples[0] = 1.0;
        bank.process(&mut samples);
        // After processing, energy should be distributed (not all zero)
        let energy: f32 = samples.iter().map(|s| s * s).sum();
        assert!(energy > 0.0, "Bank should produce output from impulse");
    }

    #[test]
    fn test_bank_reset() {
        let mut bank = BiquadBank::default_stadium_eq(48000.0);
        let mut samples = vec![1.0_f32; 100];
        bank.process(&mut samples);
        bank.reset();
        // After reset, all internal state should be zero
        for f in &bank.filters {
            assert_eq!(f.x1, 0.0);
            assert_eq!(f.x2, 0.0);
            assert_eq!(f.y1, 0.0);
            assert_eq!(f.y2, 0.0);
        }
    }
}
