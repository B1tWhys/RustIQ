mod graph;
mod sinks;

use anyhow::Result;
use flume::{Receiver, Sender};
use log::debug;
use rustiq_messages::{Command, EngineState, Event, Hertz, SourceConfig};
use rustradio::graph::{CancellationToken, GraphRunner};
use std::thread;
use std::time::Duration;

/// The SDR engine backend.
/// Owns the rustradio graph and processes commands from the UI.
pub struct Engine {
    cmd_rx: Receiver<Command>,
    event_tx: Sender<Event>,
    current_config: SourceConfig,
    should_exit: bool,
}

impl Engine {
    /// Create a new Engine instance.
    pub fn new(
        cmd_rx: Receiver<Command>,
        event_tx: Sender<Event>,
        source_config: SourceConfig,
    ) -> Self {
        debug!("Constructing a new engine");
        Self {
            cmd_rx,
            event_tx,
            current_config: source_config,
            should_exit: false,
        }
    }

    /// Run the engine (blocking).
    /// Runs in a loop that can restart the DSP graph when source changes.
    pub fn run(mut self) -> Result<()> {
        while !self.should_exit {
            self.run_graph_iteration()?;
        }
        Ok(())
    }

    fn run_graph_iteration(&mut self) -> Result<()> {
        let (graph, sample_rate_hz) =
            graph::build_graph(self.event_tx.clone(), self.current_config.clone());
        let cancel_token = graph.cancel_token();

        let state = EngineState {
            center_frequency: Hertz(0),
            sample_rate: Hertz(sample_rate_hz),
            fft_size: 4096,
            source_config: self.current_config.clone(),
        };
        self.event_tx.send(Event::StateSnapshot(state))?;

        let mut graph = graph;
        let graph_handle = thread::spawn(move || graph.run());

        self.process_commands(&cancel_token, &graph_handle);

        let _ = graph_handle.join();
        Ok(())
    }

    fn process_commands(
        &mut self,
        cancel_token: &CancellationToken,
        graph_handle: &thread::JoinHandle<std::result::Result<(), rustradio::Error>>,
    ) {
        loop {
            let msg = self.cmd_rx.recv_timeout(Duration::from_millis(100));
            debug!("Engine received message: {:?}", msg);

            match msg {
                Ok(Command::Stop) | Err(flume::RecvTimeoutError::Disconnected) => {
                    self.should_exit = true;
                    cancel_token.cancel();
                    break;
                }
                Ok(Command::ChangeSource(new_config)) => {
                    self.current_config = new_config;
                    cancel_token.cancel();
                    break;
                }
                Err(flume::RecvTimeoutError::Timeout) => {
                    if graph_handle.is_finished() {
                        self.should_exit = true;
                        break;
                    }
                }
            }
        }
    }
}
