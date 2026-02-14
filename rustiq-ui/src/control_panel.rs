use eframe::egui::{ComboBox, DragValue, Response, TextEdit, Ui, Widget};
use flume::Sender;
use std::path::PathBuf;

use rustiq_messages::{Command, Decibels, Hertz, SourceConfig};

/// Which source type is selected in the UI dropdown.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SourceType {
    SignalGenerator,
    File,
}

impl SourceType {
    fn label(&self) -> &'static str {
        match self {
            Self::SignalGenerator => "Signal Generator",
            Self::File => "IQ File",
        }
    }

    fn from_config(config: &SourceConfig) -> Self {
        match config {
            SourceConfig::SignalGenerator { .. } => Self::SignalGenerator,
            SourceConfig::File { .. } => Self::File,
        }
    }
}

/// Control panel widget for configuring the input source.
pub struct ControlPanel {
    cmd_tx: Sender<Command>,
    pending_config: SourceConfig,
    has_pending_changes: bool,
    waiting_for_apply: bool,
}

impl ControlPanel {
    pub fn new(cmd_tx: Sender<Command>) -> Self {
        Self {
            cmd_tx,
            pending_config: SourceConfig::default(),
            has_pending_changes: false,
            waiting_for_apply: false,
        }
    }

    /// Update from engine state snapshot.
    pub fn update_from_engine_state(&mut self, config: &SourceConfig) {
        self.pending_config = config.clone();
        self.has_pending_changes = false;
        self.waiting_for_apply = false;
    }

    fn current_source_type(&self) -> SourceType {
        SourceType::from_config(&self.pending_config)
    }

    fn switch_source_type(&mut self, new_type: SourceType) {
        let current_type = self.current_source_type();
        if new_type == current_type {
            return;
        }

        self.pending_config = match new_type {
            SourceType::SignalGenerator => SourceConfig::SignalGenerator {
                sample_rate: Hertz(48_000),
                signal_freq: Hertz(10_000),
                amplitude: Decibels(0.0),
            },
            SourceType::File => SourceConfig::File {
                path: PathBuf::new(),
                sample_rate: Hertz(3_200_000),
            },
        };
        self.has_pending_changes = true;
    }

    fn send_change_source(&self) {
        let _ = self
            .cmd_tx
            .send(Command::ChangeSource(self.pending_config.clone()));
    }
}

impl Widget for &mut ControlPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.heading("Input Source");
        ui.separator();

        let fields_enabled = !self.waiting_for_apply;

        // Source type selector
        let current_type = self.current_source_type();
        ui.add_enabled_ui(fields_enabled, |ui| {
            ComboBox::from_label("Source")
                .selected_text(current_type.label())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            current_type == SourceType::SignalGenerator,
                            SourceType::SignalGenerator.label(),
                        )
                        .clicked()
                    {
                        self.switch_source_type(SourceType::SignalGenerator);
                    }
                    if ui
                        .selectable_label(
                            current_type == SourceType::File,
                            SourceType::File.label(),
                        )
                        .clicked()
                    {
                        self.switch_source_type(SourceType::File);
                    }
                });
        });

        ui.add_space(10.0);

        // Source-specific controls
        ui.add_enabled_ui(fields_enabled, |ui| match &mut self.pending_config {
            SourceConfig::SignalGenerator {
                sample_rate,
                signal_freq,
                amplitude,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Sample Rate:");
                    let mut rate = sample_rate.0;
                    if ui
                        .add(DragValue::new(&mut rate).speed(1000).suffix(" Hz"))
                        .changed()
                    {
                        sample_rate.0 = rate;
                        self.has_pending_changes = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Frequency:");
                    let mut freq = signal_freq.0;
                    if ui
                        .add(DragValue::new(&mut freq).speed(100).suffix(" Hz"))
                        .changed()
                    {
                        signal_freq.0 = freq;
                        self.has_pending_changes = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Amplitude:");
                    let mut amp = amplitude.0;
                    if ui
                        .add(DragValue::new(&mut amp).speed(0.1).suffix(" dB"))
                        .changed()
                    {
                        amplitude.0 = amp;
                        self.has_pending_changes = true;
                    }
                });
            }
            SourceConfig::File { path, sample_rate } => {
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    let mut path_str = path.display().to_string();
                    if ui
                        .add(TextEdit::singleline(&mut path_str).hint_text("/path/to/file.iq"))
                        .changed()
                    {
                        *path = PathBuf::from(path_str);
                        self.has_pending_changes = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Sample Rate:");
                    let mut rate = sample_rate.0;
                    if ui
                        .add(DragValue::new(&mut rate).speed(1000).suffix(" Hz"))
                        .changed()
                    {
                        sample_rate.0 = rate;
                        self.has_pending_changes = true;
                    }
                });
            }
        });

        ui.add_space(10.0);
        ui.separator();

        // Apply button (enabled when there are changes and not waiting)
        let can_apply = self.has_pending_changes && !self.waiting_for_apply;
        ui.add_enabled_ui(can_apply, |ui| {
            if ui.button("Apply").clicked() {
                self.waiting_for_apply = true;
                self.send_change_source();
            }
        });

        ui.response()
    }
}
