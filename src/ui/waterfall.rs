use crate::ui::state::UiState;
use eframe::egui::{TextureOptions, Ui};

/// Renders the waterfall display using pre-computed pixel data from UiState.
///
/// The pixel data is pre-computed in the event handler (not during rendering),
/// so this function only needs to upload the texture when new data is available.
/// The texture handle is cached to avoid re-uploading on every frame.
pub(super) fn render(ui: &mut Ui, ui_state: &mut UiState) {
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

        // Cache the texture handle for reuse
        ui_state.waterfall_texture_handle = Some(texture);

        // Mark as uploaded to avoid redundant uploads on subsequent frames
        ui_state.waterfall_texture.mark_uploaded();
    }

    // Display the cached texture (no clone or upload)
    if let Some(texture_handle) = &ui_state.waterfall_texture_handle {
        let available_size = ui.available_size();
        ui.add(eframe::egui::Image::new(texture_handle).fit_to_exact_size(available_size));
    }
}
