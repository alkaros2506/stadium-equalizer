/// A single GRU (Gated Recurrent Unit) layer for RNNoise-style noise suppression.
pub struct GruLayer {
    input_size: usize,
    hidden_size: usize,
    // Weights for input: W_z, W_r, W_h (each: hidden_size x input_size)
    w_z: Vec<f32>,
    w_r: Vec<f32>,
    w_h: Vec<f32>,
    // Weights for hidden: U_z, U_r, U_h (each: hidden_size x hidden_size)
    u_z: Vec<f32>,
    u_r: Vec<f32>,
    u_h: Vec<f32>,
    // Biases: b_z, b_r, b_h (each: hidden_size)
    bias_z: Vec<f32>,
    bias_r: Vec<f32>,
    bias_h: Vec<f32>,
    // Hidden state
    state: Vec<f32>,
}

#[inline]
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Multiply a row-major matrix (rows x cols) by a vector (cols), writing into output (rows).
pub fn matrix_vector_multiply(
    matrix: &[f32],
    vec: &[f32],
    rows: usize,
    cols: usize,
    output: &mut [f32],
) {
    debug_assert_eq!(matrix.len(), rows * cols);
    debug_assert!(vec.len() >= cols);
    debug_assert!(output.len() >= rows);
    for (row, out) in output.iter_mut().enumerate().take(rows) {
        let mut sum = 0.0f32;
        let base = row * cols;
        for col in 0..cols {
            sum += matrix[base + col] * vec[col];
        }
        *out = sum;
    }
}

fn deterministic_weight(index: usize) -> f32 {
    let hash = (index as u64).wrapping_mul(2654435761) % 1000;
    (hash as f32) / 10000.0 - 0.05
}

impl GruLayer {
    /// Create a new GRU layer with deterministic pseudo-random weight initialization.
    pub fn new(input_size: usize, hidden_size: usize) -> Self {
        let wi_len = hidden_size * input_size;
        let wh_len = hidden_size * hidden_size;

        let mut w_z = vec![0.0f32; wi_len];
        let mut w_r = vec![0.0f32; wi_len];
        let mut w_h = vec![0.0f32; wi_len];
        let mut u_z = vec![0.0f32; wh_len];
        let mut u_r = vec![0.0f32; wh_len];
        let mut u_h = vec![0.0f32; wh_len];

        // Use different offsets so each weight matrix gets different values.
        let offsets: [usize; 6] = [
            0,
            wi_len,
            2 * wi_len,
            3 * wi_len,
            3 * wi_len + wh_len,
            3 * wi_len + 2 * wh_len,
        ];

        for i in 0..wi_len {
            w_z[i] = deterministic_weight(offsets[0] + i);
            w_r[i] = deterministic_weight(offsets[1] + i);
            w_h[i] = deterministic_weight(offsets[2] + i);
        }
        for i in 0..wh_len {
            u_z[i] = deterministic_weight(offsets[3] + i);
            u_r[i] = deterministic_weight(offsets[4] + i);
            u_h[i] = deterministic_weight(offsets[5] + i);
        }

        GruLayer {
            input_size,
            hidden_size,
            w_z,
            w_r,
            w_h,
            u_z,
            u_r,
            u_h,
            bias_z: vec![0.0; hidden_size],
            bias_r: vec![0.0; hidden_size],
            bias_h: vec![0.0; hidden_size],
            state: vec![0.0; hidden_size],
        }
    }

    /// Run one forward step of the GRU.
    ///
    /// Standard GRU equations:
    ///   z = sigmoid(W_z @ input + U_z @ state + b_z)
    ///   r = sigmoid(W_r @ input + U_r @ state + b_r)
    ///   h_candidate = tanh(W_h @ input + U_h @ (r * state) + b_h)
    ///   state = (1 - z) * state + z * h_candidate
    pub fn forward(&mut self, input: &[f32]) -> &[f32] {
        let h = self.hidden_size;

        // Scratch buffers
        let mut wi_buf = vec![0.0f32; h];
        let mut uh_buf = vec![0.0f32; h];

        // --- Update gate z ---
        matrix_vector_multiply(&self.w_z, input, h, self.input_size, &mut wi_buf);
        matrix_vector_multiply(&self.u_z, &self.state, h, h, &mut uh_buf);
        let mut z = vec![0.0f32; h];
        for i in 0..h {
            z[i] = sigmoid(wi_buf[i] + uh_buf[i] + self.bias_z[i]);
        }

        // --- Reset gate r ---
        matrix_vector_multiply(&self.w_r, input, h, self.input_size, &mut wi_buf);
        matrix_vector_multiply(&self.u_r, &self.state, h, h, &mut uh_buf);
        let mut r = vec![0.0f32; h];
        for i in 0..h {
            r[i] = sigmoid(wi_buf[i] + uh_buf[i] + self.bias_r[i]);
        }

        // --- Candidate hidden state ---
        // Compute r * state
        let mut r_state = vec![0.0f32; h];
        for i in 0..h {
            r_state[i] = r[i] * self.state[i];
        }

        matrix_vector_multiply(&self.w_h, input, h, self.input_size, &mut wi_buf);
        matrix_vector_multiply(&self.u_h, &r_state, h, h, &mut uh_buf);
        let mut h_candidate = vec![0.0f32; h];
        for i in 0..h {
            h_candidate[i] = (wi_buf[i] + uh_buf[i] + self.bias_h[i]).tanh();
        }

        // --- New state ---
        for i in 0..h {
            self.state[i] = (1.0 - z[i]) * self.state[i] + z[i] * h_candidate[i];
        }

        &self.state
    }

    /// Reset hidden state to zero.
    pub fn reset(&mut self) {
        for s in self.state.iter_mut() {
            *s = 0.0;
        }
    }

    /// Returns the hidden size of this GRU layer.
    pub fn hidden_size(&self) -> usize {
        self.hidden_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid_bounds() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-6);
        assert!(sigmoid(100.0) > 0.999);
        assert!(sigmoid(-100.0) < 0.001);
    }

    #[test]
    fn test_gru_output_size() {
        let mut gru = GruLayer::new(42, 96);
        let input = vec![0.1f32; 42];
        let out = gru.forward(&input);
        assert_eq!(out.len(), 96);
    }

    #[test]
    fn test_gru_reset() {
        let mut gru = GruLayer::new(10, 8);
        let input = vec![1.0f32; 10];
        gru.forward(&input);
        gru.reset();
        assert!(gru.state.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_matrix_vector_multiply() {
        // 2x3 matrix times 3-vector
        let mat = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let v = vec![1.0, 1.0, 1.0];
        let mut out = vec![0.0; 2];
        matrix_vector_multiply(&mat, &v, 2, 3, &mut out);
        assert!((out[0] - 6.0).abs() < 1e-6);
        assert!((out[1] - 15.0).abs() < 1e-6);
    }
}
