# V1.0 Implementation Plan

## 1. Project Setup
- [x] Add dependencies: egui/eframe, flume, rustradio, num-complex

## 2. GitHub Actions CI
- [x] Lint workflow (cargo fmt --check, cargo clippy)
- [x] Test workflow (cargo test)
- [x] Build check (cargo build)

## 3. Messages Module
- [x] Define `Command` enum
- [x] Define `Event` enum (`StateSnapshot`, `SpectrumData`)
- [x] Define `EngineState` struct
- [x] Define `Hertz` unit type

## 4. Engine Module
- [ ] `SampleSource` trait
- [ ] File-based `SampleSource` implementation (reads binary IQ files from disk)
- [ ] FFT/spectrum computation (using rustradio)
- [ ] Engine main loop: read IQ samples → compute FFT → send `SpectrumData` events
- [ ] Send `StateSnapshot` on startup

## 5. UI Module
- [ ] eframe app scaffolding
- [ ] Waterfall widget (scrolling spectrogram display)
- [ ] Event receiver loop (poll channel, update local state)

## 6. Main Entrypoint
- [ ] Create flume channels
- [ ] Spawn engine thread
- [ ] Run UI on main thread

## 7. Integration & Testing
- [ ] Boundary test with file-based `SampleSource`
- [ ] Manual testing with sample IQ files

## 8. RTL-SDR Support
- [ ] Research and select RTL-SDR crate
- [ ] RTL-SDR implementation of `SampleSource`
- [ ] Hardware testing
