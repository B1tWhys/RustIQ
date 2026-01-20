use crate::messages::EngineState;
use eframe::egui::{Color32, ColorImage, TextureOptions, Ui};
use std::collections::VecDeque;

pub(super) fn render(
    ui: &mut Ui,
    waterfall_history: &VecDeque<Vec<f32>>,
    engine_state: &EngineState,
    min_db: f32,
    max_db: f32,
) {
    if waterfall_history.is_empty() {
        ui.label("Waiting for spectrum data...");
        return;
    }

    let fft_size = engine_state.fft_size;
    let num_lines = waterfall_history.len();

    // Create image buffer
    let mut pixels = Vec::with_capacity(fft_size * num_lines);

    // Convert spectrum data to pixels
    for line in waterfall_history.iter() {
        for &magnitude in line.iter() {
            // Convert magnitude to color (grayscale)
            let intensity = magnitude_to_intensity(magnitude, min_db, max_db);
            let color = intensity_to_color(intensity);
            pixels.push(color);
        }
    }

    // Create texture
    let image = ColorImage {
        size: [fft_size, num_lines],
        source_size: [fft_size as f32, num_lines as f32].into(),
        pixels,
    };

    let texture = ui
        .ctx()
        .load_texture("waterfall", image, TextureOptions::LINEAR);

    // Display image (stretched to fill available space)
    let available_size = ui.available_size();
    ui.add(eframe::egui::Image::new(&texture).fit_to_exact_size(available_size));
}

fn magnitude_to_intensity(magnitude: f32, min_db: f32, max_db: f32) -> f32 {
    // Convert linear magnitude to dB
    if magnitude <= 0.0 {
        return 0.0; // Avoid log(0)
    }

    let db = 20.0 * magnitude.log10();

    // Avoid division by zero if range is too small
    let range = max_db - min_db;
    if range < 0.01 {
        return 0.5; // Middle gray if all values are the same
    }

    // Normalize to [0, 1] range
    ((db - min_db) / range).clamp(0.0, 1.0)
}

fn intensity_to_color(intensity: f32) -> Color32 {
    // Simple grayscale mapping
    let value = (intensity * 255.0) as u8;
    Color32::from_gray(value)
}
