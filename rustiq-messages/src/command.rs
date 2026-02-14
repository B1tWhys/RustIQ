use crate::SourceConfig;

/// Commands sent from the UI to the engine.
#[derive(Debug)]
pub enum Command {
    /// Stop the engine and terminate the DSP graph.
    Stop,
    /// Change the input source. Engine will stop current graph, rebuild, and restart.
    ChangeSource(SourceConfig),
}
