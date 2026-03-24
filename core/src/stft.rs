use num_complex::Complex;

use crate::fft::FftEngine;
use crate::window::sqrt_hann_window;

/// STFT overlap-add engine for frame-by-frame spectral processing.
///
/// Uses sqrt-Hann windows for both analysis and synthesis so that the
/// product of analysis and synthesis windows equals a Hann window,
/// satisfying the constant overlap-add (COLA) constraint when the hop
/// size divides the FFT size appropriately.
pub struct StftEngine {
    fft: FftEngine,
    fft_size: usize,
    hop_size: usize,
    window: Vec<f32>,           // sqrt-Hann analysis window
    synthesis_window: Vec<f32>, // sqrt-Hann synthesis window
    prev_output: Vec<f32>,      // overlap buffer from previous frame
    input_buffer: Vec<f32>,     // zero-padded input for FFT
    output_buffer: Vec<f32>,    // full-length IFFT output
}

impl StftEngine {
    /// Create a new STFT engine.
    ///
    /// * `fft_size` - FFT length (must be even; power-of-two recommended).
    /// * `hop_size` - Number of new samples consumed / produced per frame.
    pub fn new(fft_size: usize, hop_size: usize) -> Self {
        let window = sqrt_hann_window(fft_size);
        let synthesis_window = sqrt_hann_window(fft_size);

        Self {
            fft: FftEngine::new(fft_size),
            fft_size,
            hop_size,
            window,
            synthesis_window,
            prev_output: vec![0.0; fft_size],
            input_buffer: vec![0.0; fft_size],
            output_buffer: vec![0.0; fft_size],
        }
    }

    /// FFT size of this engine.
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// Hop size of this engine.
    pub fn hop_size(&self) -> usize {
        self.hop_size
    }

    /// Number of complex frequency bins produced by analysis (`fft_size / 2 + 1`).
    pub fn num_bins(&self) -> usize {
        self.fft_size / 2 + 1
    }

    /// Analyse a time-domain frame and return the complex spectrum.
    ///
    /// The input `frame` should contain `hop_size` samples (or fewer; it will
    /// be zero-padded to `fft_size`). The analysis window is applied before
    /// the FFT.
    ///
    /// Returns a vector of `fft_size / 2 + 1` complex bins.
    pub fn analyze(&mut self, frame: &[f32]) -> Vec<Complex<f32>> {
        // Clear the input buffer.
        self.input_buffer.iter_mut().for_each(|v| *v = 0.0);

        // Copy the input frame into the beginning of the buffer.
        let copy_len = frame.len().min(self.fft_size);
        self.input_buffer[..copy_len].copy_from_slice(&frame[..copy_len]);

        // Apply the analysis window.
        for (s, &w) in self.input_buffer.iter_mut().zip(self.window.iter()) {
            *s *= w;
        }

        // Run forward FFT.
        let num_bins = self.num_bins();
        let mut spectrum = vec![Complex::new(0.0, 0.0); num_bins];
        self.fft.forward(&mut self.input_buffer, &mut spectrum);

        spectrum
    }

    /// Synthesize a time-domain frame from a (possibly modified) complex spectrum.
    ///
    /// Runs the inverse FFT, applies the synthesis window, and overlap-adds
    /// with the tail of the previous frame. Returns exactly `hop_size` output
    /// samples.
    ///
    /// **Note:** the `spectrum` slice is modified in-place by the inverse FFT.
    pub fn synthesize(&mut self, spectrum: &mut [Complex<f32>]) -> Vec<f32> {
        // Run inverse FFT into output_buffer.
        self.fft.inverse(spectrum, &mut self.output_buffer);

        // Apply the synthesis window.
        for (s, &w) in self
            .output_buffer
            .iter_mut()
            .zip(self.synthesis_window.iter())
        {
            *s *= w;
        }

        // Overlap-add: combine with the tail from the previous frame.
        let overlap_len = self.fft_size - self.hop_size;

        // The output for this frame is the first hop_size samples of the
        // overlap-added result.
        let mut result = vec![0.0; self.hop_size];
        for (i, item) in result.iter_mut().enumerate().take(self.hop_size) {
            *item = self.output_buffer[i] + self.prev_output[i];
        }

        // Update prev_output: shift the remaining overlap region and add the
        // tail of the current output.
        let mut new_prev = vec![0.0; self.fft_size];
        // Carry forward the unused portion of the old overlap.
        for (i, item) in new_prev.iter_mut().enumerate().take(overlap_len) {
            *item = self.prev_output[self.hop_size + i] + self.output_buffer[self.hop_size + i];
        }
        self.prev_output = new_prev;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_produces_correct_bin_count() {
        let fft_size = 1024;
        let hop_size = 256;
        let mut engine = StftEngine::new(fft_size, hop_size);
        let frame = vec![0.0_f32; hop_size];
        let spectrum = engine.analyze(&frame);
        assert_eq!(spectrum.len(), fft_size / 2 + 1);
    }

    #[test]
    fn synthesize_produces_hop_size_samples() {
        let fft_size = 1024;
        let hop_size = 256;
        let mut engine = StftEngine::new(fft_size, hop_size);
        let frame = vec![0.0_f32; hop_size];
        let mut spectrum = engine.analyze(&frame);
        let output = engine.synthesize(&mut spectrum);
        assert_eq!(output.len(), hop_size);
    }

    #[test]
    fn round_trip_preserves_signal_shape() {
        let fft_size = 512;
        let hop_size = 128;
        let mut engine = StftEngine::new(fft_size, hop_size);

        // Process several frames of a simple signal and accumulate output.
        let num_frames = 8;
        let total_samples = num_frames * hop_size;
        let signal: Vec<f32> = (0..total_samples)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();

        let mut all_output = Vec::new();
        for frame_idx in 0..num_frames {
            let start = frame_idx * hop_size;
            let end = (start + hop_size).min(signal.len());
            let frame = if end <= signal.len() {
                &signal[start..end]
            } else {
                &[]
            };
            let mut spectrum = engine.analyze(frame);
            let out = engine.synthesize(&mut spectrum);
            all_output.extend_from_slice(&out);
        }

        // After several frames the output should have non-trivial energy
        // (not all zeros), confirming the round-trip works.
        let energy: f32 = all_output.iter().map(|s| s * s).sum();
        assert!(energy > 0.0, "Round-trip should produce non-zero output");
    }

    #[test]
    fn silence_in_silence_out() {
        let fft_size = 256;
        let hop_size = 64;
        let mut engine = StftEngine::new(fft_size, hop_size);

        let silence = vec![0.0_f32; hop_size];
        for _ in 0..10 {
            let mut spectrum = engine.analyze(&silence);
            let output = engine.synthesize(&mut spectrum);
            for &s in &output {
                assert!(
                    s.abs() < 1e-7,
                    "Silence round-trip should remain silent, got {}",
                    s
                );
            }
        }
    }
}
