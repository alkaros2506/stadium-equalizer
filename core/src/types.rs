use std::ops::{Deref, DerefMut};

/// Sample rate in Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRate(pub u32);

impl Deref for SampleRate {
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SampleRate {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Number of samples in one processing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSize(pub usize);

impl Deref for FrameSize {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FrameSize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// FFT length (must be a power of two).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FftSize(pub usize);

impl Deref for FftSize {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FftSize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A value expressed in decibels.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Decibels(pub f32);

impl Deref for Decibels {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Decibels {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Index of a frequency bin in a spectrum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreqBinIndex(pub usize);

impl Deref for FreqBinIndex {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FreqBinIndex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
