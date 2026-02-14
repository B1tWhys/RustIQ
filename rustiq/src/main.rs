use rustiq_engine::Engine;
use rustiq_messages::{Command, Hertz, SourceConfig};

use log::LevelFilter;
use std::io::Write;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "{:<5} - mod path |{}| - target | {} | args: |{}|",
                record.level(),
                record.module_path().unwrap_or(""),
                record.target(),
                record.args()
            )
        })
        .filter_level(LevelFilter::Debug)
        .filter_module("tracing::span", LevelFilter::Off)
        .filter_module("rustiq_engine", LevelFilter::Info)
        .filter_module("rustiq_ui", LevelFilter::Trace)
        .init();

    // Create flume channels for bidirectional communication
    let (cmd_tx, cmd_rx) = flume::unbounded();
    let (event_tx, event_rx) = flume::bounded(1);

    // Parse CLI arguments - if a file path is provided, use FileSource
    let source_config = std::env::args()
        .nth(1)
        .map(|path| SourceConfig::File {
            path: PathBuf::from(path),
            sample_rate: Hertz(3_200_000), // 3.2 MHz sample rate
        })
        .unwrap_or_default();

    // Spawn engine thread
    let engine_handle = std::thread::spawn(move || {
        let engine = Engine::new(cmd_rx, event_tx, source_config);
        engine.run().expect("Engine failed");
    });

    // Run UI on main thread (blocking)
    rustiq_ui::run(event_rx, cmd_tx.clone())?;

    // UI has exited - send stop command to engine
    let _ = cmd_tx.send(Command::Stop);

    // Wait for engine thread to finish
    engine_handle
        .join()
        .map_err(|_| anyhow::anyhow!("Engine thread panicked"))?;

    Ok(())
}
