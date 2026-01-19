# RustIQ Architecture

## Overview

RustIQ is an SDR (Software Defined Radio) receiver application similar to GQRX, built on rustradio instead of GNU Radio.

## Goals

- Cross-platform SDR receiver (Linux + macOS)
- Clean separation between UI and backend to enable future web deployment
- V1.0: Waterfall display only (no UI control, no audio)
- V1.1+: AM demodulation, UI controls for frequency/gain
- Future: FM, other modulation types, web UI with TCP streaming

## Hardware Support

- **Primary**: RTL-SDR
- **Future**: SoapySDR abstraction layer for broader device support (not critical for v1)

## Architecture

```
┌─────────────────────────────────────────────┐
│              Frontend (egui)                │
│  ┌─────────┐ ┌─────────┐ ┌───────────────┐  │
│  │Waterfall│ │Controls │ │ Audio (cpal)  │  │
│  └─────────┘ └─────────┘ └───────────────┘  │
│         │                       ↑           │
│         │    ┌──────────────────┘           │
│         ↓    ↓ (AudioChunk events)          │
│      [Local State]                          │
└─────────┬───────────────────────────────────┘
          │ Commands ↓    ↑ Events
          │ (flume)       │ (flume)
┌─────────┴───────────────┴───────────────────┐
│              Core Engine                    │
│        (no audio output, pure DSP)          │
│  ┌──────────────┐  ┌──────────────────────┐ │
│  │ SDR Device   │  │  DSP Chain           │ │
│  │ (RTL-SDR)    │→ │  (rustradio)         │ │
│  └──────────────┘  └──────────────────────┘ │
│                           ↓                 │
│  ┌──────────────┐  ┌──────────────────────┐ │
│  │ Spectrum/FFT │  │ Demodulators (AM,..) │ │
│  └──────────────┘  └──────────────────────┘ │
└─────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Frontend-Backend Communication**: Message-based protocol over flume channels
   - Two unidirectional channels (Commands down, Events up)
   - Enables future TCP/WebSocket transport for web deployment

2. **State Model**:
   - Backend sends `StateSnapshot` on connection
   - Subsequent updates via incremental `Event` messages
   - No request/response pattern - purely event-driven

3. **Audio Path**:
   - Audio samples flow through the protocol layer as `AudioChunk` events
   - cpal lives in the frontend, not the backend
   - Future: encode with Opus for network transport

4. **Threading**:
   - Engine runs on dedicated thread(s)
   - UI runs on main thread
   - Communication via async channels keeps UI responsive

## Module Structure

Single crate with strong module boundaries:

```
src/
├── main.rs              # Startup, wires frontend + engine together
├── messages/
│   ├── mod.rs           # pub types: Command, Event, EngineState
│   ├── command.rs
│   ├── event.rs
│   ├── state.rs
│   └── units.rs
├── engine/
│   ├── mod.rs           # pub: Engine::new(), Engine::run()
│   ├── graph.rs         # RustRadio graph construction (private)
│   └── sinks/
│       ├── mod.rs
│       ├── spectrum.rs  # SpectrumSink - emits SpectrumData events (private)
│       └── audio.rs     # AudioSink - emits AudioChunk events (future, private)
└── ui/
    ├── mod.rs           # pub: App::new(), run entrypoint
    ├── state.rs         # local state derived from events (private)
    ├── waterfall.rs     # waterfall widget (private)
    ├── controls.rs      # frequency, gain, mode controls (private)
    └── audio.rs         # cpal playback (private)
```

### Boundary Rules

- `messages`: All types are `pub` - this is the contract between frontend and backend
- `engine`: Only exposes `Engine::new()` and `Engine::run()`. All RustRadio implementation details (graphs, blocks, streams) are private.
- `ui`: Only exposes the app entry point. Internals are private.
- `engine` and `ui` both depend on `messages`, but never on each other

See [ENGINE_ARCHITECTURE.md](ENGINE_ARCHITECTURE.md) for detailed engine implementation using RustRadio.

## Message Types

### V1.0 (Waterfall Only)

Engine auto-starts on launch. No UI control, no audio.

```rust
// Frontend → Backend
enum Command {
    // Empty for v1.0
}

// Backend → Frontend
enum Event {
    StateSnapshot(EngineState),
    SpectrumData(Vec<f32>),
}

struct EngineState {
    center_frequency: u64,
    sample_rate: u32,
    fft_size: usize,
}
```

### Future Additions

```rust
// Frontend → Backend
enum Command {
    SetFrequency(u64),
    SetGain(f32),
    SetDemodMode(DemodMode),
    Start,
    Stop,
}

// Backend → Frontend (additional events)
enum Event {
    // ... v1.0 events plus:
    FrequencyChanged(u64),
    GainChanged(f32),
    DemodModeChanged(DemodMode),
    AudioChunk(Vec<f32>),
    DeviceStatus(DeviceStatus),
}
```

## Testing Strategy

### Boundary Tests (`tests/` directory)

Primary testing approach - test at module boundaries through public APIs:

- **Engine tests**: Construct `Engine` via public API, spawn in thread, verify Events received on flume channel
  - Use RustRadio's `SignalSource` (pure tones) for predictable FFT output
  - Validate SpectrumData events match expected frequency peaks
  - Tests interact only through the public boundary (Commands in, Events out)

- **UI tests** (future): Inject Events via mock channel, verify UI state/snapshots via egui_kittest or similar
  - Tests interact only through the UI's event consumer boundary

### Signal Sources for Testing

RustRadio provides built-in source blocks - no custom test abstractions needed:
- **`SignalSource`**: Generate pure sine waves (e.g., 10 kHz tone) - ideal for verifying FFT correctness
- **`FileSource`**: Read recorded IQ data from disk - for testing with real-world signals
- **`RtlSdrSource`**: Read from RTL-SDR hardware - production use

See [ENGINE_ARCHITECTURE.md](ENGINE_ARCHITECTURE.md) for detailed testing examples.

## Dependencies

| Crate | Purpose |
|-------|---------|
| rustradio | DSP pipeline and signal processing (includes RTL-SDR support via `rtlsdr` feature) |
| eframe | GUI framework (includes egui) |
| flume | Channel communication between engine and UI |
| num-complex | Complex number types for IQ samples |
| cpal (future) | Audio output (in frontend) |

## Future Considerations

- **Web deployment**: egui compiles to WASM; replace flume channels with WebSocket, cpal with Web Audio API
- **SoapySDR**: Abstract hardware layer for broader device support
- **Additional demodulators**: FM, SSB, etc.
- **Network streaming**: Opus encoding for audio over TCP
