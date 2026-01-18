# RustIQ

A cross-platform SDR (Software Defined Radio) receiver application, similar to GQRX but built on [rustradio](https://github.com/ThomasHabets/rustradio) instead of GNU Radio.

## Features (Planned)

- Waterfall/spectrum display
- AM demodulation (FM and others to follow)
- RTL-SDR support
- Cross-platform (Linux, macOS)

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for design decisions and module structure.

## License

TBD
