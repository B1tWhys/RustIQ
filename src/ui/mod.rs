mod state;
mod waterfall;

use crate::messages::{Command, Event};
use state::UiState;

/// Main application struct implementing the egui App trait.
pub struct RustIqApp {
    /// Receiver for events from engine
    event_rx: flume::Receiver<Event>,

    /// Sender for commands to engine (unused in v1.0)
    #[allow(dead_code)]
    cmd_tx: flume::Sender<Command>,

    /// Local application state
    state: UiState,
}

impl RustIqApp {
    fn new(event_rx: flume::Receiver<Event>, cmd_tx: flume::Sender<Command>) -> Self {
        Self {
            event_rx,
            cmd_tx,
            state: UiState::new(),
        }
    }
}

impl eframe::App for RustIqApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // 1. Pull exactly ONE event per frame (if available)
        if let Ok(event) = self.event_rx.try_recv() {
            self.state.handle_event(event);
        }

        // Always request continuous repainting for smooth 60 FPS
        ctx.request_repaint();

        // 3. Render UI
        let state = &mut self.state;
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(engine_state) = &state.engine_state {
                // Display status
                ui.label(format!(
                    "Center: {} | Rate: {} | FFT: {} | DB Range: {} to {}",
                    engine_state.center_frequency,
                    engine_state.sample_rate,
                    engine_state.fft_size,
                    state.min_db.unwrap_or_default(),
                    state.max_db.unwrap_or_default()
                ));

                ui.separator();

                // Render waterfall
                waterfall::render(ui, state);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Waiting for engine connection...");
                });
            }
        });
    }
}

/// Entry point for the UI module.
///
/// Runs the eframe application on the main thread (blocking).
pub fn run(event_rx: flume::Receiver<Event>, cmd_tx: flume::Sender<Command>) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("RustIQ"),
        ..Default::default()
    };

    eframe::run_native(
        "RustIQ",
        options,
        Box::new(|_cc| Ok(Box::new(RustIqApp::new(event_rx, cmd_tx)))),
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}
