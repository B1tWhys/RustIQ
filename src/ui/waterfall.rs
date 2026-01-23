use crate::ui::state::UiState;
use eframe::egui::{TextureOptions, Ui};

/// Renders the waterfall display using pre-computed pixel data from UiState.
///
/// The pixel data is pre-computed in the event handler (not during rendering),
/// so this function only needs to upload the texture when new data is available.
/// Uses interior mutability (Cell) to track upload state without needing &mut.
pub(super) fn render(ui: &mut Ui, ui_state: &UiState) {
    // Check if we have any image data
    if ui_state.waterfall_texture.image.pixels.is_empty() {
        ui.label("Waiting for spectrum data...");
        return;
    }

    // Only upload texture if we have new data
    if ui_state.waterfall_texture.needs_upload() {
        let texture = ui.ctx().load_texture(
            "waterfall",
            ui_state.waterfall_texture.image.clone(),
            TextureOptions::LINEAR,
        );

        // Display the texture, stretched to fill available space
        let available_size = ui.available_size();
        ui.add(eframe::egui::Image::new(&texture).fit_to_exact_size(available_size));

        // Mark as uploaded to avoid redundant uploads on subsequent frames
        ui_state.waterfall_texture.mark_uploaded();
    } else {
        // No new data - reuse cached texture
        // We still need to display it, but no GPU upload happens
        let texture = ui.ctx().load_texture(
            "waterfall",
            ui_state.waterfall_texture.image.clone(),
            TextureOptions::LINEAR,
        );
        let available_size = ui.available_size();
        ui.add(eframe::egui::Image::new(&texture).fit_to_exact_size(available_size));
    }
}
