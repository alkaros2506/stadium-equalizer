use num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::sync::Arc;

/// Wrapper around `realfft` providing forward and inverse real-valued FFT.
pub struct FftEngine {
    fft_size: usize,
    forward: Arc<dyn RealToComplex<f32> + Send + Sync>,
    inverse: Arc<dyn ComplexToReal<f32> + Send + Sync>,
    scratch_forward: Vec<Complex<f32>>,
    scratch_inverse: Vec<Complex<f32>>,
}

impl FftEngine {
    /// Create a new FFT engine for the given `fft_size` (must be even / power-of-two).
    pub fn new(fft_size: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(fft_size);
        let inverse = planner.plan_fft_inverse(fft_size);
        let scratch_forward = forward.make_scratch_vec();
        let scratch_inverse = inverse.make_scratch_vec();
        Self {
            fft_size,
            forward,
            inverse,
            scratch_forward,
            scratch_inverse,
        }
    }

    /// FFT size this engine was created with.
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// Number of complex output bins (`fft_size / 2 + 1`).
    pub fn complex_size(&self) -> usize {
        self.fft_size / 2 + 1
    }

    /// Compute the forward (real-to-complex) FFT.
    ///
    /// * `input`  – time-domain buffer of length `fft_size` (**will be modified in-place**).
    /// * `output` – complex buffer of length `fft_size / 2 + 1`.
    pub fn forward(&mut self, input: &mut [f32], output: &mut [Complex<f32>]) {
        assert_eq!(input.len(), self.fft_size);
        assert_eq!(output.len(), self.complex_size());
        self.forward
            .process_with_scratch(input, output, &mut self.scratch_forward)
            .expect("forward FFT failed");
    }

    /// Compute the inverse (complex-to-real) FFT **with normalisation**.
    ///
    /// * `input`  – complex buffer of length `fft_size / 2 + 1` (**will be modified in-place**).
    /// * `output` – time-domain buffer of length `fft_size`.
    ///
    /// The output is divided by `fft_size` so that a forward-then-inverse
    /// round-trip returns the original signal.
    pub fn inverse(&mut self, input: &mut [Complex<f32>], output: &mut [f32]) {
        assert_eq!(input.len(), self.complex_size());
        assert_eq!(output.len(), self.fft_size);
        self.inverse
            .process_with_scratch(input, output, &mut self.scratch_inverse)
            .expect("inverse FFT failed");
        let norm = 1.0 / self.fft_size as f32;
        for s in output.iter_mut() {
            *s *= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_identity() {
        let size = 1024;
        let mut engine = FftEngine::new(size);

        // Create a simple signal.
        let original: Vec<f32> = (0..size).map(|i| (i as f32).sin()).collect();
        let mut time = original.clone();
        let mut freq = vec![Complex::new(0.0, 0.0); size / 2 + 1];

        engine.forward(&mut time, &mut freq);
        engine.inverse(&mut freq, &mut time);

        for (a, b) in original.iter().zip(time.iter()) {
            assert!((a - b).abs() < 1e-4, "round-trip mismatch: {} vs {}", a, b);
        }
    }
}
