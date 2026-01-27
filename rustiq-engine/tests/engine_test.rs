use flume;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rustiq_engine::Engine;
use rustiq_messages::{Command, Event, Hertz, SourceConfig};

// Test helpers to reduce boilerplate

fn setup_engine() -> (
    flume::Sender<Command>,
    flume::Receiver<Event>,
    JoinHandle<anyhow::Result<()>>,
) {
    let (cmd_tx, cmd_rx) = flume::unbounded::<Command>();
    let (event_tx, event_rx) = flume::unbounded::<Event>();

    let handle = thread::spawn(move || {
        let engine = Engine::new(cmd_rx, event_tx, SourceConfig::default());
        engine.run()
    });

    (cmd_tx, event_rx, handle)
}

fn teardown_engine(cmd_tx: flume::Sender<Command>, handle: JoinHandle<anyhow::Result<()>>) {
    cmd_tx.send(Command::Stop).unwrap();
    let _ = handle.join();
}

fn skip_state_snapshot(event_rx: &flume::Receiver<Event>) {
    event_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Should receive StateSnapshot");
}

#[test]
fn test_engine_construction() {
    // Create channels
    let (cmd_tx, cmd_rx) = flume::unbounded::<Command>();
    let (event_tx, event_rx) = flume::unbounded::<Event>();

    // Construct engine - should not panic
    let _engine = Engine::new(cmd_rx, event_tx, SourceConfig::default());

    // Cleanup
    drop(cmd_tx);
    drop(event_rx);
}

#[test]
fn test_engine_sends_state_snapshot() {
    let (cmd_tx, event_rx, handle) = setup_engine();

    let first_event = event_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Should receive StateSnapshot");

    match first_event {
        Event::StateSnapshot(state) => {
            assert_eq!(state.sample_rate, Hertz(48_000));
            assert_eq!(state.center_frequency, Hertz(0));
            assert_eq!(state.fft_size, 4096);
        }
        _ => panic!("First event should be StateSnapshot, got {:?}", first_event),
    }

    teardown_engine(cmd_tx, handle);
}

#[test]
fn test_engine_sends_spectrum_data() {
    let (cmd_tx, event_rx, handle) = setup_engine();
    skip_state_snapshot(&event_rx);

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
    teardown_engine(cmd_tx, handle);
}

#[test]
fn test_engine_runs_without_panic() {
    let (cmd_tx, event_rx, handle) = setup_engine();

    thread::sleep(Duration::from_millis(100));
    drop(event_rx);

    cmd_tx.send(Command::Stop).unwrap();
    let result = handle.join();
    assert!(result.is_ok(), "Engine thread should not panic");
}

#[test]
fn test_fft_shows_peak_at_10khz() {
    let (cmd_tx, event_rx, handle) = setup_engine();
    skip_state_snapshot(&event_rx);

    let spectrum_data = match event_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Event::SpectrumData(data)) => data,
        Ok(other) => panic!("Expected SpectrumData, got {:?}", other),
        Err(e) => panic!("Failed to receive SpectrumData: {:?}", e),
    };

    let (max_bin_idx, _max_value) = spectrum_data
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .expect("Spectrum should not be empty");

    let sample_rate = 48_000.0;
    let fft_size = 4096.0;
    // Account for FFT shift: DC is at center (bin n/2), not at bin 0
    // Bins map to frequencies: (bin_idx - n/2) * sample_rate / n
    let peak_frequency = (max_bin_idx as f32 - fft_size / 2.0) * sample_rate / fft_size;

    let expected_frequency = 10_000.0;
    let tolerance = 100.0;
    assert!(
        (peak_frequency - expected_frequency).abs() < tolerance,
        "Peak frequency {:.1} Hz should be within {} Hz of {} Hz",
        peak_frequency,
        tolerance,
        expected_frequency
    );

    teardown_engine(cmd_tx, handle);
}
