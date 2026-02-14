mod control_panel;
mod state;
mod waterfall;

use rustiq_messages::{Command, Event};
use state::UiState;

/// Main application struct implementing the egui App trait.
pub struct RustIqApp {
    /// Receiver for events from engine
    event_rx: flume::Receiver<Event>,

    /// Local application state
    state: UiState,
}

impl RustIqApp {
    fn new(event_rx: flume::Receiver<Event>, cmd_tx: flume::Sender<Command>) -> Self {
        Self {
            event_rx,
            state: UiState::new(cmd_tx),
        }
    }
}

impl eframe::App for RustIqApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Pull events from engine
        if let Ok(event) = self.event_rx.try_recv() {
            self.state.handle_event(event);
        }

        // Always request continuous repainting for smooth 60 FPS
        ctx.request_repaint();

        // Right side panel for controls
        eframe::egui::SidePanel::right("control_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.add(&mut self.state.control_panel);
            });

        // Central panel for waterfall
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.engine_state.is_some() {
                ui.add(&mut self.state.waterfall);
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
