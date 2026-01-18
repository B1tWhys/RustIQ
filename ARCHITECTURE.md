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
│   └── state.rs
├── engine/
│   ├── mod.rs           # pub: Engine::new(), Engine::run(), channel handles
│   ├── sdr.rs           # RTL-SDR interface (private)
│   ├── dsp.rs           # rustradio pipeline (private)
│   ├── demod/
│   │   ├── mod.rs
│   │   └── am.rs        # AM demodulator (private)
│   └── spectrum.rs      # FFT computation (private)
└── ui/
    ├── mod.rs           # pub: App::new(), run entrypoint
    ├── state.rs         # local state derived from events (private)
    ├── waterfall.rs     # waterfall widget (private)
    ├── controls.rs      # frequency, gain, mode controls (private)
    └── audio.rs         # cpal playback (private)
```

### Boundary Rules

- `messages`: All types are `pub` - this is the contract between frontend and backend
- `engine`: Only exposes constructor and channel handles. Internals are private.
- `ui`: Only exposes the app entry point. Internals are private.
- `engine` and `ui` both depend on `messages`, but never on each other

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

Primary testing approach - test at module boundaries:

- **Engine tests**: Mock SDR input (provide known IQ data), verify demodulated audio output
- **UI tests**: Mock channel (inject Events), verify UI state/snapshots via egui_kittest or similar

### Unit Tests (inline `#[cfg(test)]`)

Secondary - only for complex DSP logic where fine-grained testing adds value.

### SDR Abstraction for Testing

```rust
trait SampleSource {
    fn read_samples(&mut self, buf: &mut [Complex<f32>]) -> Result<usize>;
}
```

- Production: RTL-SDR implementation
- Tests: Mock that emits known signals (pure tones, pre-recorded IQ data)

## Dependencies (Planned)

| Crate | Purpose |
|-------|---------|
| rustradio | DSP pipeline and signal processing |
| rtlsdr (TBD) | RTL-SDR hardware interface |
| egui + eframe | GUI framework |
| cpal | Audio output (in frontend) |
| flume | Channel communication |

## Future Considerations

- **Web deployment**: egui compiles to WASM; replace flume channels with WebSocket, cpal with Web Audio API
- **SoapySDR**: Abstract hardware layer for broader device support
- **Additional demodulators**: FM, SSB, etc.
- **Network streaming**: Opus encoding for audio over TCP
