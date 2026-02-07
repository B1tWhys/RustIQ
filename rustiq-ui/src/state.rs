use crate::waterfall::Waterfall;
use rustiq_messages::{EngineState, Event};

/// Local UI state derived from engine events.
pub(super) struct UiState {
    /// Current engine state (from StateSnapshot)
    pub engine_state: Option<EngineState>,

    /// Waterfall widget state
    pub waterfall: Waterfall,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            engine_state: None,
            waterfall: Waterfall::new(),
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::StateSnapshot(state) => {
                self.engine_state = Some(state);
            }
            Event::SpectrumData(data) => {
                self.waterfall.insert_spectrum_line(&data);
            }
        }
    }
}
