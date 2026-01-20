# V1.0 Implementation Plan

## 1. Project Setup
- [x] Add dependencies: egui/eframe, flume, rustradio, num-complex
- [x] Create project structure with messages, engine, ui modules

## 2. GitHub Actions CI
- [x] Lint workflow (cargo fmt --check, cargo clippy)
- [x] Test workflow (cargo test)
- [x] Build check (cargo build)

## 3. Messages Module
- [x] Define `Command` enum
- [x] Define `Event` enum (`StateSnapshot`, `SpectrumData`)
- [x] Define `EngineState` struct
- [x] Define `Hertz` unit type

## 4. Engine Module - Basic Structure

### 4.1 Remove Old Code
- [x] Remove `IqSource` trait and `FileIqSource` from `engine/sdr.rs`
- [x] Delete `engine/sdr.rs` entirely (using RustRadio's source blocks instead)

### 4.2 Implement SpectrumSink
- [x] Create `engine/sinks/` directory structure
- [x] Implement `SpectrumSink` in `engine/sinks/spectrum.rs`
  - Implement `Block` trait for RustRadio
  - Consume f32 data from input stream
  - Emit `Event::SpectrumData` via flume channel (use `try_send`)

### 4.3 Test: Engine Construction
- [x] Create `tests/engine_test.rs`
- [x] Write test: construct `Engine::new(cmd_rx, event_tx)` (will initially fail to compile)

### 4.4 Implement Engine Public API
- [x] Implement `Engine` struct in `engine/mod.rs` (holds Graph, cmd_rx)
- [x] Implement `Engine::new(cmd_rx, event_tx)` constructor
  - Call `graph::build_graph(event_tx)` to create graph
- [x] Test from 4.3 now compiles

### 4.5 Test: Engine Runs
- [x] Update test to call `engine.run()` in spawned thread
- [x] Test should compile but will fail at runtime (no graph implementation yet)

### 4.6 Implement Basic Graph (No FFT)
- [x] Create `engine/graph.rs`
- [x] Implement `build_graph(event_tx)` function
  - Create `SignalSource` (10 kHz @ 48 kHz sample rate)
  - Connect directly to `SpectrumSink` (temporary - will add FFT later)
  - Return configured `Graph`

### 4.7 Implement Engine::run()
- [x] Implement `Engine::run()` method in `engine/mod.rs`
  - Send `StateSnapshot` on startup via event_tx
  - Call `self.graph.run()`
- [x] Test from 4.5 now passes (receives events)

### 4.8 Test: Verify StateSnapshot
- [x] Write test: verify first event is `StateSnapshot`
- [x] Verify `EngineState` contains correct sample_rate, center_frequency

### 4.9 Test: Verify SpectrumData Events
- [x] Write test: verify `SpectrumData` events are received
- [x] Test receives multiple events successfully

## 5. Engine Module - Add FFT Processing

### 5.1 Add FFT to Graph
- [x] Update `build_graph()` to insert FFT block between SignalSource and SpectrumSink
  - Research which rustradio FFT block to use
  - Configure FFT size (4096)
  - Connect: SignalSource → FFT → SpectrumSink

### 5.2 Test: Verify FFT Output
- [x] Write test: verify FFT shows peak at 10 kHz
  - Receive SpectrumData event
  - Find peak bin in spectrum
  - Calculate frequency of peak (bin_index * sample_rate / fft_size)
  - Assert peak is within 100 Hz of 10 kHz

## 6. UI Module
- [x] eframe app scaffolding in `ui/mod.rs`
- [x] Implement event receiver loop (poll event_rx, update local state)
- [x] Waterfall widget in `ui/waterfall.rs` (scrolling spectrogram display)
- [x] Wire waterfall to display SpectrumData events

## 7. Main Entrypoint
- [x] Create flume channels in `main.rs`
- [x] Construct `Engine` with channels
- [x] Spawn engine thread with `std::thread::spawn(|| engine.run())`
- [x] Create and run UI on main thread

## 8. Manual Verification
- [ ] Run application with `cargo run --release`
- [ ] Verify waterfall display shows 10 kHz spike
- [ ] Verify UI remains responsive

## Future: FileSource Support
- [ ] Add configuration parameter to `build_graph()` for source type
- [ ] Support `FileSource` for testing with recorded IQ data
- [ ] Create/record test IQ files

## Future: RTL-SDR Support
- [ ] Enable `rtlsdr` feature in rustradio dependency
- [ ] Add `RtlSdrSource` + `RtlSdrDecode` path in `build_graph()`
- [ ] Hardware testing with RTL-SDR device
