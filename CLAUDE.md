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

The application uses a **Cargo workspace** with four crates:

- **`rustiq-messages`** - Shared protocol types (Command, Event, EngineState) forming the contract between frontend and backend. Zero external dependencies.
- **`rustiq-engine`** - Backend DSP processing library (rustradio pipeline, RTL-SDR interface, demodulators, FFT). Only exposes `Engine` struct with constructor and `run()` method.
- **`rustiq-ui`** - Frontend egui application library. Only exposes `run()` function as entry point.
- **`rustiq`** - Main binary crate that integrates all workspace members.

**Critical rule**: `rustiq-engine` and `rustiq-ui` both depend on `rustiq-messages`, but never on each other. The workspace structure enforces this at compile time.

**Import paths**: Use workspace crate names in imports:
- `use rustiq_messages::{Command, Event, Hertz, SourceConfig};`
- `use rustiq_engine::Engine;`
- `rustiq_ui::run(event_rx, cmd_tx)?;`

Communication between engine and UI uses two unidirectional flume channels (Commands down, Events up). Backend sends a `StateSnapshot` on connection, then incremental events. No request/response pattern.

Audio samples flow through the protocol layer as `AudioChunk` events - cpal lives in the frontend, not the backend.

## Logging

Use the `log` crate facade (`debug!`, `info!`, `warn!`, `error!`) for all logging. The binary crate initializes `env_logger`, controlled at runtime via the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run --release   # All debug+ messages
RUST_LOG=rustiq_engine=debug cargo run --release  # Filter to one crate
```

## Testing Strategy

- **Integration tests**: Each library crate has its own `tests/` directory (e.g., `rustiq-engine/tests/engine_test.rs`) for testing public APIs
- **Unit tests** (inline `#[cfg(test)]`): Only for complex DSP logic

See docs/ARCHITECTURE.md for detailed design decisions.
