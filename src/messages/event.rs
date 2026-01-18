use super::EngineState;

/// Events sent from the engine to the UI.
pub enum Event {
    /// Initial state snapshot sent on connection.
    StateSnapshot(EngineState),
    /// FFT magnitude data for waterfall display.
    SpectrumData(Vec<f32>),
}
