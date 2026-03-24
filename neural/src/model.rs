use crate::feature_extract::{FeatureExtractor, BARK_BANDS};
use crate::gru::{matrix_vector_multiply, sigmoid, GruLayer};

pub struct DenseLayer {
    weights: Vec<f32>, // output_size x input_size, row-major
    biases: Vec<f32>,  // output_size
    input_size: usize,
    output_size: usize,
}

fn dense_deterministic_weight(index: usize) -> f32 {
    let hash = (index as u64)
        .wrapping_mul(1664525)
        .wrapping_add(1013904223)
        % 1000;
    (hash as f32) / 10000.0 - 0.05
}

impl DenseLayer {
    /// Create a dense layer with deterministic pseudo-random weight initialization.
    pub fn new(input_size: usize, output_size: usize) -> Self {
        let len = output_size * input_size;
        let mut weights = vec![0.0f32; len];
        for (i, w) in weights.iter_mut().enumerate().take(len) {
            *w = dense_deterministic_weight(i);
        }
        DenseLayer {
            weights,
            biases: vec![0.0; output_size],
            input_size,
            output_size,
        }
    }

    /// Compute output = weights @ input + biases.
    pub fn forward(&self, input: &[f32], output: &mut [f32]) {
        matrix_vector_multiply(
            &self.weights,
            input,
            self.output_size,
            self.input_size,
            output,
        );
        for (out, bias) in output
            .iter_mut()
            .zip(self.biases.iter())
            .take(self.output_size)
        {
            *out += bias;
        }
    }

    /// Forward pass with tanh activation.
    pub fn forward_tanh(&self, input: &[f32], output: &mut [f32]) {
        self.forward(input, output);
        for item in output.iter_mut().take(self.output_size) {
            *item = item.tanh();
        }
    }

    /// Forward pass with sigmoid activation.
    pub fn forward_sigmoid(&self, input: &[f32], output: &mut [f32]) {
        self.forward(input, output);
        for item in output.iter_mut().take(self.output_size) {
            *item = sigmoid(*item);
        }
    }
}

pub struct RNNoiseModel {
    feature_extractor: FeatureExtractor,
    dense_input: DenseLayer,  // 42 -> 96
    gru1: GruLayer,           // 96 -> 96
    gru2: GruLayer,           // 96 -> 48
    gru3: GruLayer,           // 48 -> 48
    dense_output: DenseLayer, // 48 -> 22
    output_gains: [f32; BARK_BANDS],
}

impl RNNoiseModel {
    /// Create a new RNNoise model with all layers initialized.
    pub fn new() -> Self {
        RNNoiseModel {
            feature_extractor: FeatureExtractor::new(),
            dense_input: DenseLayer::new(42, 96),
            gru1: GruLayer::new(96, 96),
            gru2: GruLayer::new(96, 48),
            gru3: GruLayer::new(48, 48),
            dense_output: DenseLayer::new(48, BARK_BANDS),
            output_gains: [0.0; BARK_BANDS],
        }
    }

    /// Process one 10ms frame.
    ///
    /// Takes the power spectrum (at least 513 bins) and returns 22 Bark-band gains in [0, 1].
    pub fn process_frame(&mut self, power_spectrum: &[f32]) -> &[f32; BARK_BANDS] {
        // 1. Extract 42 features
        let features = self.feature_extractor.extract(power_spectrum);

        // 2. Dense input (tanh): 42 -> 96
        let mut dense1_out = vec![0.0f32; 96];
        self.dense_input.forward_tanh(&features, &mut dense1_out);

        // 3. GRU1: 96 -> 96
        let gru1_out = self.gru1.forward(&dense1_out).to_vec();

        // 4. GRU2: 96 -> 48
        let gru2_out = self.gru2.forward(&gru1_out).to_vec();

        // 5. GRU3: 48 -> 48
        let gru3_out = self.gru3.forward(&gru2_out).to_vec();

        // 6. Dense output (sigmoid): 48 -> 22
        let mut gains = [0.0f32; BARK_BANDS];
        self.dense_output.forward_sigmoid(&gru3_out, &mut gains);

        self.output_gains = gains;
        &self.output_gains
    }

    /// Reset all GRU hidden states (e.g., at the start of a new audio stream).
    pub fn reset(&mut self) {
        self.gru1.reset();
        self.gru2.reset();
        self.gru3.reset();
    }
}

impl Default for RNNoiseModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dense_layer_dimensions() {
        let dense = DenseLayer::new(10, 5);
        let input = vec![1.0f32; 10];
        let mut output = vec![0.0f32; 5];
        dense.forward(&input, &mut output);
        assert_eq!(output.len(), 5);
    }

    #[test]
    fn test_model_output_gains_count() {
        let mut model = RNNoiseModel::new();
        let spectrum = vec![0.01f32; 513];
        let gains = model.process_frame(&spectrum);
        assert_eq!(gains.len(), BARK_BANDS);
    }

    #[test]
    fn test_model_output_gains_in_range() {
        let mut model = RNNoiseModel::new();
        let spectrum = vec![0.5f32; 513];
        let gains = model.process_frame(&spectrum);
        for &g in gains.iter() {
            assert!((0.0..=1.0).contains(&g), "Gain {} out of sigmoid range", g);
        }
    }

    #[test]
    fn test_model_reset() {
        let mut model = RNNoiseModel::new();
        let spectrum = vec![1.0f32; 513];
        model.process_frame(&spectrum);
        model.reset();
        // After reset, processing again should give deterministic results
        let gains = model.process_frame(&spectrum);
        assert_eq!(gains.len(), BARK_BANDS);
    }
}
