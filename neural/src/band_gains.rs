use crate::feature_extract::{BARK_BANDS, BARK_BAND_EDGES};

pub struct BandGainInterpolator {
    num_bins: usize,
    interpolated_gains: Vec<f32>,
}

impl BandGainInterpolator {
    /// Create an interpolator for the given number of FFT bins.
    pub fn new(num_bins: usize) -> Self {
        BandGainInterpolator {
            num_bins,
            interpolated_gains: vec![0.0f32; num_bins],
        }
    }

    /// Interpolate 22 band gains into per-bin gains.
    ///
    /// Within each band the gain is constant. At band boundaries, the gain is
    /// linearly interpolated between the adjacent bands. Bins beyond the last
    /// band edge receive the gain of the last band.
    pub fn interpolate(&mut self, band_gains: &[f32; BARK_BANDS]) -> &[f32] {
        // Precompute band center bins (midpoint of each band).
        let mut band_centers = [0usize; BARK_BANDS];
        for (b, bc) in band_centers.iter_mut().enumerate().take(BARK_BANDS) {
            *bc = (BARK_BAND_EDGES[b] + BARK_BAND_EDGES[b + 1]) / 2;
        }

        for (bin, ig) in self
            .interpolated_gains
            .iter_mut()
            .enumerate()
            .take(self.num_bins)
        {
            // Find which band this bin belongs to.
            let band = find_band(bin);

            match band {
                Some(b) => {
                    let center = band_centers[b];
                    if bin == center || BARK_BANDS <= 1 {
                        // Exactly at the center: use band gain directly.
                        *ig = band_gains[b];
                    } else if bin < center && b > 0 {
                        // Between previous band center and this band center:
                        // linearly interpolate.
                        let prev_center = band_centers[b - 1];
                        if center == prev_center {
                            *ig = band_gains[b];
                        } else {
                            let t = (bin as f32 - prev_center as f32)
                                / (center as f32 - prev_center as f32);
                            *ig = band_gains[b - 1] * (1.0 - t) + band_gains[b] * t;
                        }
                    } else if bin > center && b < BARK_BANDS - 1 {
                        // Between this band center and next band center:
                        // linearly interpolate.
                        let next_center = band_centers[b + 1];
                        if next_center == center {
                            *ig = band_gains[b];
                        } else {
                            let t =
                                (bin as f32 - center as f32) / (next_center as f32 - center as f32);
                            *ig = band_gains[b] * (1.0 - t) + band_gains[b + 1] * t;
                        }
                    } else {
                        // Edge cases: first band before center or last band after center.
                        *ig = band_gains[b];
                    }
                }
                None => {
                    // Beyond all band edges: use the last band gain.
                    *ig = band_gains[BARK_BANDS - 1];
                }
            }
        }

        &self.interpolated_gains[..self.num_bins]
    }
}

/// Find which Bark band a given FFT bin falls into.
/// Returns None if the bin is beyond the last band edge.
fn find_band(bin: usize) -> Option<usize> {
    (0..BARK_BANDS).find(|&b| bin < BARK_BAND_EDGES[b + 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolator_length() {
        let mut interp = BandGainInterpolator::new(513);
        let gains = [0.5f32; BARK_BANDS];
        let result = interp.interpolate(&gains);
        assert_eq!(result.len(), 513);
    }

    #[test]
    fn test_uniform_gains() {
        let mut interp = BandGainInterpolator::new(513);
        let gains = [0.8f32; BARK_BANDS];
        let result = interp.interpolate(&gains);
        // When all band gains are equal, every bin should have that gain.
        for &g in result.iter() {
            assert!((g - 0.8).abs() < 1e-6, "Expected 0.8, got {}", g);
        }
    }

    #[test]
    fn test_find_band() {
        assert_eq!(find_band(0), Some(0));
        assert_eq!(find_band(3), Some(0));
        assert_eq!(find_band(4), Some(1));
        assert_eq!(find_band(512), Some(21));
        assert_eq!(find_band(513), None);
    }

    #[test]
    fn test_gains_in_range() {
        let mut interp = BandGainInterpolator::new(513);
        let mut gains = [0.0f32; BARK_BANDS];
        for (i, gain) in gains.iter_mut().enumerate().take(BARK_BANDS) {
            *gain = i as f32 / (BARK_BANDS - 1) as f32;
        }
        let result = interp.interpolate(&gains);
        for &g in result.iter() {
            assert!((0.0..=1.0).contains(&g), "Gain {} out of range", g);
        }
    }
}
