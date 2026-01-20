use flume::Sender;
use rustradio::graph::{Graph, GraphRunner};
use rustradio::blocks::SignalSourceFloat;

use crate::messages::Event;
use super::sinks::SpectrumSink;

/// Build the DSP graph for the engine.
/// Currently creates a simple signal source → spectrum sink pipeline.
pub fn build_graph(event_tx: Sender<Event>) -> Graph {
    // Create a 10 kHz signal source at 48 kHz sample rate
    let sample_rate = 48_000.0;
    let signal_freq = 10_000.0;
    let amplitude = 1.0;

    let (signal_source, prev) = SignalSourceFloat::new(sample_rate, signal_freq, amplitude);

    // Create spectrum sink
    let spectrum_sink = SpectrumSink::new(prev, event_tx.clone());

    let mut graph = Graph::new();

    // Add blocks to graph
    graph.add(Box::new(signal_source));
    graph.add(Box::new(spectrum_sink));

    graph
}
