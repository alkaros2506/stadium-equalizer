use std::f32::consts::PI;

/// Generate a square-root Hann window of the given `size`.
///
/// w[n] = sqrt(0.5 * (1 - cos(2 * pi * n / size)))  for n in 0..size
pub fn sqrt_hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|n| {
            let x = 0.5 * (1.0 - (2.0 * PI * n as f32 / size as f32).cos());
            x.sqrt()
        })
        .collect()
}

/// Generate a standard Hann window of the given `size`.
///
/// w[n] = 0.5 * (1 - cos(2 * pi * n / size))  for n in 0..size
pub fn hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|n| 0.5 * (1.0 - (2.0 * PI * n as f32 / size as f32).cos()))
        .collect()
}

/// Apply a window function in-place to a sample buffer.
///
/// `samples` and `window` must have the same length (or `window` may be longer;
/// only the first `samples.len()` coefficients are used).
pub fn apply_window(samples: &mut [f32], window: &[f32]) {
    for (s, &w) in samples.iter_mut().zip(window.iter()) {
        *s *= w;
    }
}

/// Remove (undo) a window function in-place.
///
/// Bins where the window value is below `floor` are left unchanged to avoid
/// division by near-zero.
pub fn remove_window(samples: &mut [f32], window: &[f32], floor: f32) {
    for (s, &w) in samples.iter_mut().zip(window.iter()) {
        if w.abs() > floor {
            *s /= w;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hann_endpoints_are_zero() {
        let w = hann_window(256);
        assert!(w[0].abs() < 1e-7);
    }

    #[test]
    fn sqrt_hann_squared_equals_hann() {
        let sh = sqrt_hann_window(256);
        let h = hann_window(256);
        for (a, b) in sh.iter().zip(h.iter()) {
            assert!((a * a - b).abs() < 1e-6);
        }
    }
}
