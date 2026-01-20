use flume;
use std::thread;
use std::time::Duration;

use RustIQ::engine::Engine;
use RustIQ::messages::{Command, Event, Hertz};

#[test]
fn test_engine_construction() {
    // Create channels
    let (cmd_tx, cmd_rx) = flume::unbounded::<Command>();
    let (event_tx, event_rx) = flume::unbounded::<Event>();

    // Construct engine - should not panic
    let _engine = Engine::new(cmd_rx, event_tx);

    // Cleanup
    drop(cmd_tx);
    drop(event_rx);
}

#[test]
fn test_engine_sends_state_snapshot() {
    // Create channels
    let (_cmd_tx, cmd_rx) = flume::unbounded::<Command>();
    let (event_tx, event_rx) = flume::unbounded::<Event>();

    // Create and run engine in background thread
    let handle = thread::spawn(move || {
        let engine = Engine::new(cmd_rx, event_tx);
        engine.run()
    });

    // Wait for and verify first event is StateSnapshot
    let first_event = event_rx.recv_timeout(Duration::from_secs(2))
        .expect("Should receive StateSnapshot");

    match first_event {
        Event::StateSnapshot(state) => {
            assert_eq!(state.sample_rate, Hertz(48_000));
            assert_eq!(state.center_frequency, Hertz(0));
            assert_eq!(state.fft_size, 4096);
        }
        _ => panic!("First event should be StateSnapshot, got {:?}", first_event),
    }

    // Cleanup: drop receiver to stop engine
    drop(event_rx);
    let _ = handle.join();
}

#[test]
fn test_engine_sends_spectrum_data() {
    // Create channels
    let (_cmd_tx, cmd_rx) = flume::unbounded::<Command>();
    let (event_tx, event_rx) = flume::unbounded::<Event>();

    // Create and run engine in background thread
    let handle = thread::spawn(move || {
        let engine = Engine::new(cmd_rx, event_tx);
        engine.run()
    });

    // Skip the initial StateSnapshot
    let _state_snapshot = event_rx.recv_timeout(Duration::from_secs(2))
        .expect("Should receive StateSnapshot");

    // Verify we receive SpectrumData events
    let mut spectrum_count = 0;
    for _ in 0..5 {
        match event_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Event::SpectrumData(data)) => {
                assert!(!data.is_empty(), "Spectrum data should not be empty");
                spectrum_count += 1;
            }
            Ok(Event::StateSnapshot(_)) => {
                panic!("Should not receive another StateSnapshot");
            }
            Err(e) => {
                panic!("Failed to receive SpectrumData: {:?}", e);
            }
        }
    }

    assert_eq!(spectrum_count, 5, "Should receive 5 SpectrumData events");

    // Cleanup
    drop(event_rx);
    let _ = handle.join();
}

#[test]
fn test_engine_runs_without_panic() {
    // Create channels
    let (_cmd_tx, cmd_rx) = flume::unbounded::<Command>();
    let (event_tx, event_rx) = flume::unbounded::<Event>();

    // Create and run engine
    let handle = thread::spawn(move || {
        let engine = Engine::new(cmd_rx, event_tx);
        engine.run()
    });

    // Let it run briefly
    thread::sleep(Duration::from_millis(100));

    // Cleanup - dropping event_rx should cause engine to terminate
    drop(event_rx);

    // Engine should finish without panicking
    let result = handle.join();
    assert!(result.is_ok(), "Engine thread should not panic");
}
