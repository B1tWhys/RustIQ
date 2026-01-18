# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RustIQ is an SDR (Software Defined Radio) receiver application similar to GQRX, built on rustradio instead of GNU Radio. Target platforms are Linux and macOS.

## Build Commands

```bash
cargo build           # Debug build
cargo build --release # Release build
cargo run --release   # Run the application
cargo test            # Run all tests
cargo test <name>     # Run specific test
```

## Development Workflow

This project uses trunk-based development. Always work on a feature branch and submit changes via pull request to `main`.

## Architecture

The application has three main modules with strict boundaries:

- **`messages`** - Public types (Command, Event, EngineState) forming the contract between frontend and backend
- **`engine`** - Backend DSP processing (rustradio pipeline, RTL-SDR interface, demodulators, FFT). Only exposes constructor and channel handles.
- **`ui`** - Frontend (egui GUI, cpal audio playback). Only exposes app entry point.

**Critical rule**: `engine` and `ui` both depend on `messages`, but never on each other.

Communication between engine and UI uses two unidirectional flume channels (Commands down, Events up). Backend sends a `StateSnapshot` on connection, then incremental events. No request/response pattern.

Audio samples flow through the protocol layer as `AudioChunk` events - cpal lives in the frontend, not the backend.

## Testing Strategy

- **Boundary tests** (`tests/` directory): Mock SDR input for engine tests, mock channels for UI tests
- **Unit tests** (inline `#[cfg(test)]`): Only for complex DSP logic

See ARCHITECTURE.md for detailed design decisions.
