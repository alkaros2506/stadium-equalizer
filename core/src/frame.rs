use num_complex::Complex;

use crate::config::MAX_FRAME_SIZE;
use crate::types::SampleRate;

// ---------------------------------------------------------------------------
// AudioFrame
// ---------------------------------------------------------------------------

/// A fixed-capacity audio frame backed by a stack-allocated array.
#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// Sample data (only the first `len` entries are meaningful).
    pub data: [f32; MAX_FRAME_SIZE],
    /// Number of valid samples in `data`.
    pub len: usize,
    /// Sample rate associated with this frame.
    pub sample_rate: SampleRate,
}

impl AudioFrame {
    /// Create a new, silent audio frame.
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            data: [0.0; MAX_FRAME_SIZE],
            len: 0,
            sample_rate,
        }
    }

    /// Create a frame pre-filled with `len` zeros.
    pub fn zeroed(len: usize, sample_rate: SampleRate) -> Self {
        assert!(len <= MAX_FRAME_SIZE, "len exceeds MAX_FRAME_SIZE");
        Self {
            data: [0.0; MAX_FRAME_SIZE],
            len,
            sample_rate,
        }
    }

    /// Create a frame by copying from a slice.
    pub fn from_slice(samples: &[f32], sample_rate: SampleRate) -> Self {
        assert!(
            samples.len() <= MAX_FRAME_SIZE,
            "slice length exceeds MAX_FRAME_SIZE"
        );
        let mut data = [0.0; MAX_FRAME_SIZE];
        data[..samples.len()].copy_from_slice(samples);
        Self {
            data,
            len: samples.len(),
            sample_rate,
        }
    }

    /// Return a slice over the valid samples.
    pub fn as_slice(&self) -> &[f32] {
        &self.data[..self.len]
    }

    /// Return a mutable slice over the valid samples.
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data[..self.len]
    }
}

// ---------------------------------------------------------------------------
// SpectrumFrame
// ---------------------------------------------------------------------------

/// A frequency-domain frame (complex spectrum).
#[derive(Debug, Clone)]
pub struct SpectrumFrame {
    /// Complex spectral bins.
    pub data: Vec<Complex<f32>>,
    /// Number of valid bins.
    pub len: usize,
    /// FFT size that produced this spectrum.
    pub fft_size: usize,
}

impl SpectrumFrame {
    /// Create a zeroed spectrum for a given FFT size.
    /// The number of bins is `fft_size / 2 + 1` (real-valued FFT).
    pub fn new(fft_size: usize) -> Self {
        let len = fft_size / 2 + 1;
        Self {
            data: vec![Complex::new(0.0, 0.0); len],
            len,
            fft_size,
        }
    }

    /// Create a spectrum from an existing vector of complex bins.
    pub fn from_vec(bins: Vec<Complex<f32>>, fft_size: usize) -> Self {
        let len = bins.len();
        Self {
            data: bins,
            len,
            fft_size,
        }
    }

    /// Return a slice over the valid bins.
    pub fn as_slice(&self) -> &[Complex<f32>] {
        &self.data[..self.len]
    }

    /// Return a mutable slice over the valid bins.
    pub fn as_mut_slice(&mut self) -> &mut [Complex<f32>] {
        &mut self.data[..self.len]
    }

    /// Compute the power spectrum (magnitude squared per bin).
    pub fn power_spectrum(&self) -> Vec<f32> {
        self.data[..self.len].iter().map(|c| c.norm_sqr()).collect()
    }

    /// Compute the magnitude spectrum.
    pub fn magnitude_spectrum(&self) -> Vec<f32> {
        self.data[..self.len].iter().map(|c| c.norm()).collect()
    }
}

// ---------------------------------------------------------------------------
// RingBuffer
// ---------------------------------------------------------------------------

/// A simple ring buffer with dynamic backing storage.
#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    buf: Vec<T>,
    write_pos: usize,
    read_pos: usize,
    capacity: usize,
    /// Number of samples currently available for reading.
    count: usize,
}

impl<T: Copy + Default> RingBuffer<T> {
    /// Create a ring buffer that can hold `capacity` items.
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: vec![T::default(); capacity],
            write_pos: 0,
            read_pos: 0,
            capacity,
            count: 0,
        }
    }

    /// Push a single sample into the buffer.
    /// If the buffer is full the oldest sample is overwritten.
    pub fn push(&mut self, sample: T) {
        self.buf[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.capacity;
        if self.count == self.capacity {
            // Overwrite: advance read pointer too.
            self.read_pos = (self.read_pos + 1) % self.capacity;
        } else {
            self.count += 1;
        }
    }

    /// Push a slice of samples into the buffer.
    pub fn push_slice(&mut self, samples: &[T]) {
        for &s in samples {
            self.push(s);
        }
    }

    /// Read up to `count` samples into `output`.
    /// Returns the number of samples actually read.
    pub fn read(&mut self, output: &mut [T], count: usize) -> usize {
        let to_read = count.min(self.count).min(output.len());
        for item in output.iter_mut().take(to_read) {
            *item = self.buf[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.capacity;
        }
        self.count -= to_read;
        to_read
    }

    /// Number of samples available for reading.
    pub fn available(&self) -> usize {
        self.count
    }

    /// Reset the buffer to empty.
    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.read_pos = 0;
        self.count = 0;
    }
}
