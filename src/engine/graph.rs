use std::borrow::Cow;

use flume::Sender;
use rustradio::graph::{Graph, GraphRunner};
use rustradio::blocks::{FftStream, Map, SignalSourceComplex};

use crate::messages::Event;
use super::sinks::SpectrumSink;

/// Build the DSP graph for the engine.
/// Currently creates a simple signal source → spectrum sink pipeline.
pub fn build_graph(event_tx: Sender<Event>) -> Graph {
    // Create a 10 kHz signal source at 48 kHz sample rate
    let sample_rate = 48_000.0;
    let signal_freq = 10_000.0;
    let amplitude = 1.0;

    let (signal_source, prev) = SignalSourceComplex::new(sample_rate, signal_freq, amplitude);

    // Create fft block
    let fft_size = 4096;
    let (fft, prev) = FftStream::new(prev, fft_size);

    // Compute magnitude from complex FFT output
    let (map_magnitude, prev) = Map::new(prev, "MapMagnitude", |sample, tags| (sample.norm(), Cow::Borrowed(tags)));

    // Create spectrum sink
    let spectrum_sink = SpectrumSink::new(prev, event_tx.clone(), fft_size);

    let mut graph = Graph::new();

    // Add blocks to graph
    graph.add(Box::new(signal_source));
    graph.add(Box::new(fft));
    graph.add(Box::new(map_magnitude));
    graph.add(Box::new(spectrum_sink));

    graph
}
