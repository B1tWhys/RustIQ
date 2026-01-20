use crate::messages::{EngineState, Event};
use std::collections::VecDeque;

/// Local UI state derived from engine events.
pub(super) struct UiState {
    /// Current engine state (from StateSnapshot)
    pub engine_state: Option<EngineState>,

    /// Waterfall history buffer
    /// Each entry is one FFT frame (Vec<f32>)
    /// Newest data at index 0, oldest at the end
    pub waterfall_history: VecDeque<Vec<f32>>,

    /// Maximum number of waterfall lines to keep
    pub waterfall_max_lines: usize,

    /// Minimum dB value seen in recent data (for dynamic scaling)
    pub min_db: f32,

    /// Maximum dB value seen in recent data (for dynamic scaling)
    pub max_db: f32,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            engine_state: None,
            waterfall_history: VecDeque::with_capacity(512),
            waterfall_max_lines: 512, // ~10 seconds at 50 FPS
            min_db: 0.0,
            max_db: 1.0, // Initialize to avoid division by zero
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::StateSnapshot(state) => {
                self.engine_state = Some(state);
            }
            Event::SpectrumData(data) => {
                // Update min/max dB range for dynamic scaling
                self.update_db_range(&data);

                // Add new data at front
                self.waterfall_history.push_front(data);

                // Trim old data from back
                while self.waterfall_history.len() > self.waterfall_max_lines {
                    self.waterfall_history.pop_back();
                }
            }
        }
    }

    fn update_db_range(&mut self, data: &[f32]) {
        if data.is_empty() {
            return;
        }

        // Convert magnitudes to dB, filtering out zeros
        let db_values: Vec<f32> = data
            .iter()
            .filter(|&&mag| mag > 0.0) // Avoid log(0)
            .map(|&mag| 20.0 * mag.log10())
            .collect();

        if db_values.is_empty() {
            return;
        }

        self.min_db = db_values.iter().copied().fold(f32::INFINITY, f32::min);
        self.max_db = db_values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    }
}
