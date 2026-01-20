use rustiq::{engine::Engine, messages::Command, ui};

fn main() -> anyhow::Result<()> {
    // Create flume channels for bidirectional communication
    let (cmd_tx, cmd_rx) = flume::unbounded();
    let (event_tx, event_rx) = flume::unbounded();

    // Spawn engine thread
    let engine_handle = std::thread::spawn(move || {
        let engine = Engine::new(cmd_rx, event_tx);
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
