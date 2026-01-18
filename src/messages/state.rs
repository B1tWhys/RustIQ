use super::Hertz;

/// Current state of the SDR engine.
pub struct EngineState {
    /// Center frequency
    pub center_frequency: Hertz,
    /// Sample rate
    pub sample_rate: Hertz,
    /// FFT size (number of bins)
    pub fft_size: usize,
}
