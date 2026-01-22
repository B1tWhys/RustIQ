use rustiq::{
    engine::Engine,
    messages::{Command, Hertz, SourceConfig},
    ui,
};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // Create flume channels for bidirectional communication
    let (cmd_tx, cmd_rx) = flume::unbounded();
    let (event_tx, event_rx) = flume::bounded(1); // flume::unbounded();

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
    ui::run(event_rx, cmd_tx.clone())?;

    // UI has exited - send stop command to engine
    let _ = cmd_tx.send(Command::Stop);

    // Wait for engine thread to finish
    engine_handle
        .join()
        .map_err(|_| anyhow::anyhow!("Engine thread panicked"))?;

    Ok(())
}
