use std::borrow::Cow;

use flume::Sender;
use rustradio::blocks::{FftStream, FileSource, Map, SignalSourceComplex};
use rustradio::graph::{Graph, GraphRunner};

use super::sinks::SpectrumSink;
use crate::messages::{Event, SourceConfig};

/// Build the DSP graph for the engine.
/// Returns (Graph, sample_rate_hz).
pub fn build_graph(event_tx: Sender<Event>, source_config: SourceConfig) -> (Graph, u64) {
    let (prev, sample_rate, mut graph) = match source_config {
        SourceConfig::SignalGenerator {
            sample_rate,
            signal_freq,
            amplitude,
        } => {
            let (signal_source, prev) = SignalSourceComplex::new(
                sample_rate.as_hz() as f32,
                signal_freq.as_hz() as f32,
                amplitude.to_linear(),
            );
            let mut g = Graph::new();
            g.add(Box::new(signal_source));
            (prev, sample_rate.as_hz(), g)
        }
        SourceConfig::File { path, sample_rate } => {
            let (file_source, prev) = FileSource::new(path).expect("Failed to open IQ file");
            let mut g = Graph::new();
            g.add(Box::new(file_source));
            (prev, sample_rate.as_hz(), g)
        }
    };

    // Create fft block
    let fft_size = 4096;
    let (fft, prev) = FftStream::new(prev, fft_size);

    // Compute magnitude from complex FFT output
    let (map_magnitude, prev) = Map::new(prev, "MapMagnitude", |sample, tags| {
        (sample.norm(), Cow::Borrowed(tags))
    });

    // Create spectrum sink
    let spectrum_sink = SpectrumSink::new(prev, event_tx.clone(), fft_size);

    // Add blocks to graph
    graph.add(Box::new(fft));
    graph.add(Box::new(map_magnitude));
    graph.add(Box::new(spectrum_sink));

    (graph, sample_rate)
}
