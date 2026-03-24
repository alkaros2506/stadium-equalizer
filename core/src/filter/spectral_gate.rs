use num_complex::Complex;

/// Applies gain masks to complex spectra and provides gain-smoothing utilities.
pub struct SpectralGate;

impl SpectralGate {
    /// Apply a gain mask to a complex spectrum in-place.
    ///
    /// For each bin k, `spectrum[k] *= gain[k]`.
    /// The shorter of the two slices determines how many bins are processed.
    pub fn apply(spectrum: &mut [Complex<f32>], gains: &[f32]) {
        for (s, &g) in spectrum.iter_mut().zip(gains.iter()) {
            *s *= g;
        }
    }

    /// Combine multiple gain vectors element-wise by multiplication.
    ///
    /// `output[k] = gains[0][k] * gains[1][k] * ... * gains[N-1][k]`
    /// The output length determines how many bins are processed. Each gain
    /// slice must be at least as long as `output`.
    pub fn combine_gains(gains: &[&[f32]], output: &mut [f32]) {
        // Initialize output to 1.0
        for v in output.iter_mut() {
            *v = 1.0;
        }
        for gain_vec in gains {
            for (o, &g) in output.iter_mut().zip(gain_vec.iter()) {
                *o *= g;
            }
        }
    }

    /// Smooth gains temporally to reduce musical noise artifacts.
    ///
    /// `output[k] = alpha * prev[k] + (1 - alpha) * current[k]`
    ///
    /// * `alpha` near 1.0 means more smoothing (slow adaptation).
    /// * `alpha` near 0.0 means less smoothing (fast adaptation).
    ///
    /// All slices must be at least as long as `output`.
    pub fn smooth_gains(prev: &[f32], current: &[f32], alpha: f32, output: &mut [f32]) {
        for ((o, &p), &c) in output.iter_mut().zip(prev.iter()).zip(current.iter()) {
            *o = alpha * p + (1.0 - alpha) * c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_gain() {
        let mut spectrum = vec![
            Complex::new(2.0, 1.0),
            Complex::new(0.0, 4.0),
            Complex::new(1.0, -1.0),
        ];
        let gains = [0.5, 0.0, 1.0];
        SpectralGate::apply(&mut spectrum, &gains);
        assert!((spectrum[0].re - 1.0).abs() < 1e-6);
        assert!((spectrum[0].im - 0.5).abs() < 1e-6);
        assert!((spectrum[1].re).abs() < 1e-6);
        assert!((spectrum[1].im).abs() < 1e-6);
        assert!((spectrum[2].re - 1.0).abs() < 1e-6);
        assert!((spectrum[2].im - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_combine_gains() {
        let g1 = [0.5, 0.8, 1.0];
        let g2 = [0.4, 0.5, 0.1];
        let mut output = [0.0; 3];
        SpectralGate::combine_gains(&[&g1, &g2], &mut output);
        assert!((output[0] - 0.2).abs() < 1e-6);
        assert!((output[1] - 0.4).abs() < 1e-6);
        assert!((output[2] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_combine_gains_empty() {
        let mut output = [0.0; 3];
        SpectralGate::combine_gains(&[], &mut output);
        // No gain vectors -> output stays at 1.0
        assert!((output[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_smooth_gains() {
        let prev = [1.0, 0.0, 0.5];
        let current = [0.0, 1.0, 0.5];
        let mut output = [0.0; 3];
        SpectralGate::smooth_gains(&prev, &current, 0.8, &mut output);
        // output[0] = 0.8*1.0 + 0.2*0.0 = 0.8
        assert!((output[0] - 0.8).abs() < 1e-6);
        // output[1] = 0.8*0.0 + 0.2*1.0 = 0.2
        assert!((output[1] - 0.2).abs() < 1e-6);
        // output[2] = 0.8*0.5 + 0.2*0.5 = 0.5
        assert!((output[2] - 0.5).abs() < 1e-6);
    }
}
