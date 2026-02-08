use flume::Sender;

use crate::control_panel::ControlPanel;
use crate::waterfall::Waterfall;
use rustiq_messages::{Command, EngineState, Event};

/// Local UI state derived from engine events.
pub(super) struct UiState {
    /// Current engine state (from StateSnapshot)
    pub engine_state: Option<EngineState>,

    /// Waterfall widget state
    pub waterfall: Waterfall,

    /// Control panel widget state
    pub control_panel: ControlPanel,
}

impl UiState {
    pub fn new(cmd_tx: Sender<Command>) -> Self {
        Self {
            engine_state: None,
            waterfall: Waterfall::new(),
            control_panel: ControlPanel::new(cmd_tx),
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::StateSnapshot(state) => {
                self.control_panel
                    .update_from_engine_state(&state.source_config);
                self.engine_state = Some(state);
            }
            Event::SpectrumData(data) => {
                self.waterfall.insert_spectrum_line(&data);
            }
        }
    }
}
