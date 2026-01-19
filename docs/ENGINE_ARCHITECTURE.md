# Engine Architecture

This document describes the detailed architecture of RustIQ's engine module, including how it uses the [rustradio](https://docs.rs/rustradio/latest/rustradio/) library for DSP processing.

## RustRadio Overview

RustRadio is a digital signal processing framework inspired by GNU Radio, written in Rust. It provides a **graph-based processing model** where:

- Signal processing happens through **blocks** connected by **unidirectional streams**
- Each block has zero or more input streams and zero or more output streams
- Signal flows from **sources** (blocks with no inputs) through processing blocks to **sinks** (blocks with no outputs)
- Streams are **type-safe** - the output type of one block must match the input type of the next

### Key Advantages
- Type safety prevents wiring errors at compile time
- Performance comparable to C++ (faster than Python-based solutions)
- Rust's ownership model prevents data races in signal processing chains
- Both synchronous (`Graph`) and multithreaded (`MtGraph`) execution models

## Architecture Fit

RustRadio **is** the DSP processing core within the `engine` module:

```
┌─────────────────────────────────────────────┐
│              Frontend (egui)                │
│                                             │
└─────────┬───────────────────────────────────┘
          │ Commands ↓    ↑ Events
          │ (flume)       │ (flume)
┌─────────┴───────────────┴───────────────────┐
│          Engine Module (Public API)         │
│           Engine::new(), run()              │
│  ┌───────────────────────────────────────┐  │
│  │     RustRadio Graph (Private)         │  │
│  │                                       │  │
│  │  [Source] → [Process] → [Sinks]       │  │
│  │     ↓           ↓          ↓          │  │
│  │  SignalGen    FFT      Event emit     │  │
│  │  FileSource   Demod                   │  │
│  │  RTL-SDR                              │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### Module Boundary

The `engine` module exposes a **minimal public API**:
- `Engine::new(cmd_rx, event_tx)` - Constructor
- `Engine::run()` - Blocking execution (caller spawns thread)

All RustRadio graph details (blocks, streams, connections) are **private implementation details** inside the engine module.

Main's responsibility:
```rust
// In main.rs
let (cmd_tx, cmd_rx) = flume::unbounded();
let (event_tx, event_rx) = flume::unbounded();

let engine = Engine::new(cmd_rx, event_tx);

// Spawn engine thread
std::thread::spawn(move || {
    engine.run().expect("Engine error");
});

// Run UI on main thread with event_rx and cmd_tx
run_ui(event_rx, cmd_tx);
```

## Block-Based Pipeline Pattern

### Basic Pattern

RustRadio applications follow this structure:

```rust
use rustradio::graph::Graph;

// 1. Create a graph
let mut graph = Graph::new();

// 2. Create blocks, which return (block, stream_handle)
let (source_block, source_stream) = SourceBlock::new(/* config */);

// 3. Connect blocks by passing previous stream to next block
let (processing_block, processed_stream) = ProcessingBlock::new(source_stream, /* config */);

// 4. Terminal blocks (sinks) consume the stream
let sink_block = SinkBlock::new(processed_stream);

// 5. Add blocks to graph
graph.add(Box::new(source_block));
graph.add(Box::new(processing_block));
graph.add(Box::new(sink_block));

// 6. Run the graph (blocking)
graph.run()?;
```

### Stream Typing

Streams are strongly typed using `Stream<T>`:
- `Stream<Complex<f32>>` - IQ samples
- `Stream<f32>` - Real-valued samples (e.g., demodulated audio)
- `Stream<u8>` - Byte data

The compiler ensures blocks are connected with compatible types.

## Key RustRadio Blocks

Based on the [rustradio documentation](https://docs.rs/rustradio/latest/rustradio/), here are the blocks most relevant to RustIQ:

### Source Blocks
- **`SignalSource`** - Generate pure sine wave (ideal for initial development)
- **`FileSource`** - Read raw IQ samples from file
- **`RtlSdrSource`** - Read from RTL-SDR hardware (requires `rtlsdr` feature)
- **`RtlSdrDecode`** - Convert RTL-SDR byte format to `Complex<f32>` I/Q samples

### Processing Blocks
- **`fft_filter`** - FFT-based filtering (efficient for many taps)
- **`fir`** - Finite impulse response filter
- **`rational_resampler`** - Fractional resampling (e.g., 2.4 MSPS → 48 kHz)
- **`quadrature_demod`** - FM demodulator core
- **`complex_to_mag2`** - Magnitude squared (for power spectrum)
- **`hilbert`** - Hilbert transform (for SSB)

### Sink Blocks (Built-in)
- **`DebugSink`** - Print values (debugging)
- **`FileSink`** - Write to file
- **Custom sinks** - We'll implement these to emit Events

## Development Progression

### Phase 1: SignalSource (Current)

Start with `SignalSource` to generate a pure sine wave:

```rust
use rustradio::blocks::SignalSource;

let frequency = 10_000.0; // 10 kHz tone
let sample_rate = 48_000.0; // 48 kHz
let (source, stream) = SignalSource::new(frequency, sample_rate);
```

**Why this is ideal for initial development:**
- **Predictable output**: Pure sine wave shows a single spike in FFT
- **No hardware needed**: Works on any machine
- **No file dependencies**: Self-contained signal generation
- **Easy verification**: FFT should show a spike at exactly 10 kHz

### Phase 2: FileSource (Testing)

Switch to `FileSource` for testing with real-world signals:

```rust
use rustradio::blocks::FileSource;

let (source, stream) = FileSource::new("tests/data/fm_broadcast.iq");
```

Benefits:
- Test with complex modulated signals
- Reproducible test cases
- No hardware dependency during development

### Phase 3: RtlSdrSource (Production)

Final step: read from RTL-SDR hardware:

```rust
use rustradio::blocks::{RtlSdrSource, RtlSdrDecode};

let center_freq = 100_000_000u64; // 100 MHz
let sample_rate = 2_400_000u32;   // 2.4 MSPS
let (source, stream) = RtlSdrSource::new(center_freq, sample_rate);
let (decode, stream) = RtlSdrDecode::new(stream);
```

## Data Flow Examples

### V1.0: Waterfall with SignalSource

```
SignalSource (10 kHz sine @ 48 kHz sample rate)
   ↓ (Stream<Complex<f32>>)
FFT processing block
   ↓ (Stream<f32>) - power spectrum
SpectrumSink (custom) → sends Event::SpectrumData to UI
```

FFT output will show a clear spike at 10 kHz - easy to verify correctness.

### Future: Waterfall + Audio with RTL-SDR

```
RtlSdrSource
   ↓
RtlSdrDecode
   ↓ (Stream<Complex<f32>>)
   ├─→ FFT → SpectrumSink → Event::SpectrumData
   └─→ Filter → Demod → AudioSink → Event::AudioChunk
```

## Custom Sink Implementation

We'll implement custom sink blocks that extract processed data and emit Events to the UI:

```rust
use rustradio::block::{Block, BlockResult};
use rustradio::stream::Streamp;
use crate::messages::Event;

pub struct SpectrumSink {
    input: Streamp<f32>,
    event_tx: flume::Sender<Event>,
}

impl SpectrumSink {
    pub fn new(input: Streamp<f32>, event_tx: flume::Sender<Event>) -> Self {
        Self { input, event_tx }
    }
}

impl Block for SpectrumSink {
    fn work(&mut self) -> Result<BlockResult> {
        // Read all available FFT data from input stream
        let fft_data = self.input.consume_all();

        if fft_data.is_empty() {
            return Ok(BlockResult::Noop);
        }

        // Send event to UI (non-blocking)
        let _ = self.event_tx.try_send(Event::SpectrumData(fft_data));

        Ok(BlockResult::Ok)
    }
}
```

Key points:
- Custom sinks implement the `Block` trait
- `work()` method is called by RustRadio's execution engine
- Use `try_send()` to avoid blocking if UI is slow
- Return `BlockResult::Ok` to continue processing

## Engine Module Structure

The engine module encapsulates all RustRadio details:

```
src/engine/
├── mod.rs              # Public API: Engine struct, new(), run()
├── graph.rs            # Private: build_graph() function
└── sinks/
    ├── mod.rs
    ├── spectrum.rs     # SpectrumSink - emits SpectrumData events
    └── audio.rs        # AudioSink - emits AudioChunk events (future)
```

### Public API (engine/mod.rs)

```rust
pub struct Engine {
    graph: Graph,
    cmd_rx: flume::Receiver<Command>,
}

impl Engine {
    pub fn new(cmd_rx: flume::Receiver<Command>, event_tx: flume::Sender<Event>) -> Self {
        let graph = graph::build_graph(event_tx);
        Self { graph, cmd_rx }
    }

    pub fn run(mut self) -> Result<()> {
        // Send initial state snapshot
        // ...

        // Run the graph (blocking)
        self.graph.run()
    }
}
```

### Private Graph Builder (engine/graph.rs)

```rust
use rustradio::graph::Graph;
use rustradio::blocks::SignalSource;
use crate::messages::Event;
use super::sinks::SpectrumSink;

pub(super) fn build_graph(event_tx: flume::Sender<Event>) -> Graph {
    let mut g = Graph::new();

    // Configuration
    let signal_freq = 10_000.0;   // 10 kHz tone
    let sample_rate = 48_000.0;   // 48 kHz sampling

    // Build pipeline
    let (source, stream) = SignalSource::new(signal_freq, sample_rate);
    // ... FFT processing ...
    let sink = SpectrumSink::new(stream, event_tx);

    // Add to graph
    g.add(Box::new(source));
    // ... add processing blocks ...
    g.add(Box::new(sink));

    g
}
```

This keeps all RustRadio implementation details private to the engine module.

## Threading Model

### V1.0: Single-Threaded Graph

Use `Graph` (synchronous, single-threaded):

```rust
// engine/mod.rs
impl Engine {
    pub fn run(mut self) -> Result<()> {
        self.graph.run() // Blocking, runs all blocks sequentially
    }
}

// main.rs
let engine = Engine::new(cmd_rx, event_tx);

std::thread::spawn(move || {
    engine.run().expect("Engine failed");
});
```

Main spawns the thread, but engine handles graph execution internally.

### Future: Multithreaded Graph

When pipeline has independent branches (FFT + demodulation):

```rust
use rustradio::graph::MtGraph;

pub(super) fn build_graph(event_tx: flume::Sender<Event>) -> MtGraph {
    let mut g = MtGraph::new();
    // ... each block can run on separate thread ...
    g
}
```

Change return type from `Graph` to `MtGraph` - API stays the same.

## Configuration and Control

### Static Configuration (V1.0)

All parameters hardcoded in `build_graph()`:

```rust
pub(super) fn build_graph(event_tx: flume::Sender<Event>) -> Graph {
    // Static configuration
    let signal_freq = 10_000.0;
    let sample_rate = 48_000.0;
    let fft_size = 4096;

    // Build pipeline with these parameters
    // ...
}
```

### Dynamic Reconfiguration (V1.1+)

When implementing `Command` handling:

```rust
impl Engine {
    pub fn run(mut self) -> Result<()> {
        loop {
            // Check for commands (non-blocking)
            if let Ok(cmd) = self.cmd_rx.try_recv() {
                match cmd {
                    Command::SetFrequency(freq) => {
                        // Rebuild graph with new frequency
                        self.graph.stop();
                        self.graph = build_graph_with_config(freq, self.event_tx);
                        // Continue running
                    }
                    // ... other commands ...
                }
            }

            // Continue processing
            self.graph.work()?; // Process one iteration
        }
    }
}
```

Note: This requires more sophisticated graph management. V1.0 can just run until completion.

## Testing Strategy

Focus on **integration testing** through the Engine's public API. Tests should construct an `Engine`, spawn it in a thread, and verify the events received on the flume channel match expectations.

### Basic Integration Test

Test that the engine sends expected events:

```rust
#[test]
fn test_engine_sends_events() {
    let (cmd_tx, cmd_rx) = flume::unbounded();
    let (event_tx, event_rx) = flume::unbounded();

    let engine = Engine::new(cmd_rx, event_tx);

    // Run engine in background thread
    std::thread::spawn(move || {
        engine.run()
    });

    // Verify StateSnapshot arrives first
    let event = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(matches!(event, Event::StateSnapshot(_)));

    // Verify SpectrumData arrives
    let event = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(matches!(event, Event::SpectrumData(_)));
}
```

### Verifying SignalSource FFT Output

With `SignalSource` generating a 10 kHz tone, the FFT output should show a clear peak at 10 kHz:

```rust
#[test]
fn test_signal_source_produces_correct_fft() {
    let (cmd_tx, cmd_rx) = flume::unbounded();
    let (event_tx, event_rx) = flume::unbounded();

    let engine = Engine::new(cmd_rx, event_tx);

    std::thread::spawn(move || {
        engine.run()
    });

    // Skip StateSnapshot
    let _ = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    // Get SpectrumData event
    let event = event_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let spectrum = match event {
        Event::SpectrumData(data) => data,
        _ => panic!("Expected SpectrumData"),
    };

    // Find peak in spectrum
    let peak_bin = spectrum.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap();

    // Calculate frequency of peak bin
    let sample_rate = 48_000.0;
    let fft_size = 4096;
    let bin_width = sample_rate / fft_size as f32;
    let peak_freq = peak_bin as f32 * bin_width;

    // Should be close to 10 kHz
    assert!((peak_freq - 10_000.0).abs() < 100.0,
            "Expected peak at 10 kHz, found peak at {} Hz", peak_freq);
}
```

This validates the entire pipeline (SignalSource → FFT → SpectrumSink → Event emission) is working correctly.

## RTL-SDR Integration (Future)

### Dependencies

Enable RTL-SDR support in `Cargo.toml`:

```toml
[dependencies]
rustradio = { version = "0.8", features = ["rtlsdr"] }
```

This pulls in the `rtlsdr` crate and requires `librtlsdr.so` at runtime.

### Using RtlSdrSource

In `graph.rs`, change source block:

```rust
// Development: SignalSource
let (source, stream) = SignalSource::new(10_000.0, 48_000.0);

// Production: RtlSdrSource
use rustradio::blocks::{RtlSdrSource, RtlSdrDecode};
let center_freq = 100_000_000u64;
let sample_rate = 2_400_000u32;
let (source, stream) = RtlSdrSource::new(center_freq, sample_rate);
let (decode, stream) = RtlSdrDecode::new(stream);
```

Configuration can be passed to `build_graph()` to switch between sources.

## Performance Considerations

### Buffer Sizes

RustRadio processes data in chunks. Configure at block creation:

- **Waterfall**: 4096-8192 samples (balances update rate with efficiency)
- **Audio**: 512-1024 samples (lower latency for real-time playback)
- **FFT**: Powers of 2 (2048, 4096, 8192) for optimal performance

### FFT Backend

RustRadio supports multiple FFT implementations:

- **Default**: Pure Rust FFT (no extra dependencies)
- **`libfftw` feature**: GPL-licensed, potentially faster for large transforms

For V1.0, use the default. Profile before adding FFTW dependency.

## Migration Path

### V1.0: Waterfall with SignalSource
- [ ] Remove `IqSource` trait and `FileIqSource` from `engine/sdr.rs`
- [ ] Implement `SpectrumSink` custom block
- [ ] Implement `build_graph()` with `SignalSource` → FFT → `SpectrumSink`
- [ ] Implement `Engine::new()` and `Engine::run()` public API
- [ ] Integration tests: construct Engine, verify events on flume channel
- [ ] Wire up UI to display spectrum

### V1.1: Switch to FileSource
- [ ] Add `FileSource` support in `build_graph()`
- [ ] Create test IQ files (record from hardware or generate with tools)
- [ ] Test with real-world signals

### V1.2: RTL-SDR Support
- [ ] Add `rtlsdr` feature to dependencies
- [ ] Add `RtlSdrSource` + `RtlSdrDecode` path in `build_graph()`
- [ ] Hardware testing

### V1.3+: Audio & Dynamic Control
- [ ] Implement `AudioSink` custom block
- [ ] Add demodulator blocks (AM via `quadrature_demod`)
- [ ] Branch pipeline: FFT path + demod path
- [ ] Implement `Command` handling in `Engine::run()`
- [ ] Consider `MtGraph` if performance bottleneck

## Key Takeaways

1. **Use `SignalSource` for initial development** - produces easily interpretable FFT output
2. **Engine module encapsulates all RustRadio details** - main only spawns thread
3. **Implement only custom sinks** - to bridge RustRadio to UI events
4. **Start simple with `Graph`** - single-threaded is fine for V1.0
5. **RustRadio handles all DSP** - FFT, filtering, demodulation all provided
6. **Public API is minimal** - `Engine::new()` and `Engine::run()` only

This approach maintains clean module boundaries while minimizing custom code.

## Resources

- [RustRadio Documentation](https://docs.rs/rustradio/latest/rustradio/)
- [RustRadio GitHub](https://github.com/ThomasHabets/rustradio)
- [RustRadio on crates.io](https://crates.io/crates/rustradio)

---

*This document will be updated as we gain practical experience integrating RustRadio into RustIQ.*
