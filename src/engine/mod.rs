mod graph;
mod sinks;

use anyhow::Result;
use flume::{Receiver, Sender};
use rustradio::graph::{CancellationToken, Graph, GraphRunner};
use std::thread;
use std::time::Duration;

use crate::messages::{Command, EngineState, Event, Hertz, SourceConfig};

/// The SDR engine backend.
/// Owns the rustradio graph and processes commands from the UI.
pub struct Engine {
    graph: Graph,
    cmd_rx: Receiver<Command>,
    event_tx: Sender<Event>,
    cancellation_token: CancellationToken,
    sample_rate: Hertz,
}

impl Engine {
    /// Create a new Engine instance.
    pub fn new(
        cmd_rx: Receiver<Command>,
        event_tx: Sender<Event>,
        source_config: SourceConfig,
    ) -> Self {
        let (graph, sample_rate_hz) = graph::build_graph(event_tx.clone(), source_config);
        let token = graph.cancel_token();
        Self {
            graph,
            cmd_rx,
            event_tx,
            cancellation_token: token,
            sample_rate: Hertz(sample_rate_hz),
        }
    }

    /// Run the engine (blocking).
    /// Sends initial StateSnapshot, then runs the DSP graph on a separate thread
    /// while processing commands on the main thread.
    pub fn run(self) -> Result<()> {
        // Send initial state snapshot
        let initial_state = EngineState {
            center_frequency: Hertz(0),
            sample_rate: self.sample_rate,
            fft_size: 4096,
        };
        self.event_tx.send(Event::StateSnapshot(initial_state))?;

        // Spawn graph on separate thread
        let mut graph = self.graph;
        let graph_handle = thread::spawn(move || graph.run());

        // Command processing loop on main thread
        loop {
            // Poll for commands with timeout to check if graph thread is still alive
            match self.cmd_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Command::Stop) | Err(flume::RecvTimeoutError::Disconnected) => {
                    // Stop command received or channel closed - cancel the graph
                    self.cancellation_token.cancel();
                    break;
                }
                Err(flume::RecvTimeoutError::Timeout) => {
                    // Check if graph thread has finished
                    if graph_handle.is_finished() {
                        break;
                    }
                    // Continue waiting
                }
            }
        }

        // Wait for graph thread to finish and return its result
        let graph_result = graph_handle
            .join()
            .map_err(|_| anyhow::anyhow!("Graph thread panicked"))?;

        // Convert rustradio::Error to anyhow::Error
        graph_result.map_err(|e| anyhow::anyhow!("Graph error: {}", e))
    }
}
