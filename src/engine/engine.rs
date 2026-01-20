use anyhow::Result;
use flume::{Receiver, Sender};
use rustradio::graph::{Graph, GraphRunner};

use crate::messages::{Command, Event, EngineState, Hertz};

use super::graph;

/// The SDR engine backend.
/// Owns the rustradio graph and processes commands from the UI.
pub struct Engine {
    graph: Graph,
    cmd_rx: Receiver<Command>,
    event_tx: Sender<Event>,
}

impl Engine {
    /// Create a new Engine instance.
    pub fn new(cmd_rx: Receiver<Command>, event_tx: Sender<Event>) -> Self {
        let graph = graph::build_graph(event_tx.clone());
        Self {
            graph,
            cmd_rx,
            event_tx,
        }
    }

    /// Run the engine (blocking).
    /// Sends initial StateSnapshot, then runs the DSP graph.
    pub fn run(mut self) -> Result<()> {
        // Send initial state snapshot
        let initial_state = EngineState {
            center_frequency: Hertz(0),
            sample_rate: Hertz(48_000),
            fft_size: 4096,
        };
        self.event_tx.send(Event::StateSnapshot(initial_state))?;

        // Run the graph (blocking)
        self.graph.run()?;

        Ok(())
    }
}
