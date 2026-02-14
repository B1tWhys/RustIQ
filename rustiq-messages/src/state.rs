use crate::{Decibels, Hertz};
use std::path::PathBuf;

/// Current state of the SDR engine.
#[derive(Debug)]
pub struct EngineState {
    /// Center frequency
    pub center_frequency: Hertz,
    /// Sample rate
    pub sample_rate: Hertz,
    /// FFT size (number of bins)
    pub fft_size: usize,
    /// Current source configuration
    pub source_config: SourceConfig,
}

/// Configuration for the SDR signal source.
#[derive(Debug, Clone)]
pub enum SourceConfig {
    /// Generate a test signal (sine wave at specified frequency).
    SignalGenerator {
        sample_rate: Hertz,
        signal_freq: Hertz,
        amplitude: Decibels,
    },
    /// Read IQ samples from a file.
    File { path: PathBuf, sample_rate: Hertz },
}

impl Default for SourceConfig {
    fn default() -> Self {
        SourceConfig::SignalGenerator {
            sample_rate: Hertz(48_000),
            signal_freq: Hertz(10_000),
            amplitude: Decibels(0.0), // 0 dB = amplitude 1.0
        }
    }
}
