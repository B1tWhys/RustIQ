use eframe::egui::{Color32, ColorImage, TextureHandle};

use rustiq_messages::{Decibels, EngineState, Event};
use std::cell::Cell;

/// Encapsulates the waterfall texture and GPU upload state.
///
/// The image pixels are updated in the event handler when new spectrum data arrives.
/// The `needs_gpu_upload` flag tracks whether the texture needs to be re-uploaded to
/// the GPU, avoiding redundant uploads when rendering multiple frames without new data.
///
/// We use `Cell<bool>` for interior mutability, allowing the render function to mark
/// the texture as uploaded even with only a `&` reference.
pub(super) struct WaterfallTexture {
    pub image: ColorImage,
    needs_gpu_upload: Cell<bool>,
}

impl WaterfallTexture {
    pub fn new() -> Self {
        Self {
            image: ColorImage::default(),
            needs_gpu_upload: Cell::new(false),
        }
    }

    /// Mark that new pixel data is available and needs GPU upload
    pub fn mark_updated(&mut self) {
        self.needs_gpu_upload.set(true);
    }

    /// Mark that texture has been uploaded to GPU (callable with & reference)
    pub fn mark_uploaded(&self) {
        self.needs_gpu_upload.set(false);
    }

    /// Check if texture needs GPU upload
    pub fn needs_upload(&self) -> bool {
        self.needs_gpu_upload.get()
    }
}

/// Local UI state derived from engine events.
pub(super) struct UiState {
    /// Current engine state (from StateSnapshot)
    pub engine_state: Option<EngineState>,

    /// Maximum number of waterfall lines to keep in the texture
    pub waterfall_max_lines: usize,

    /// Minimum value seen in recent data (for dynamic scaling)
    pub min_db: Option<f32>,

    /// Maximum value seen in recent data (for dynamic scaling)
    pub max_db: Option<f32>,

    /// Waterfall texture state (pre-computed pixels + GPU upload tracking)
    pub waterfall_texture: WaterfallTexture,

    /// Cached texture handle to avoid re-uploading on every frame
    pub waterfall_texture_handle: Option<TextureHandle>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            engine_state: None,
            waterfall_max_lines: 1024,
            min_db: None,
            max_db: None,
            waterfall_texture: WaterfallTexture::new(),
            waterfall_texture_handle: None,
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::StateSnapshot(state) => {
                self.engine_state = Some(state);
            }
            Event::SpectrumData(data) => {
                self.handle_spectrum_update(&data);
            }
        }
    }

    fn handle_spectrum_update(&mut self, data: &[f32]) {
        // Update min/max dB range for dynamic scaling
        self.update_db_range(data);

        // Add new data at front
        self.insert_spectrum_line(data);
    }

    fn update_db_range(&mut self, data: &[f32]) {
        if data.is_empty() {
            return;
        }

        // Update min/max values seen
        let min_data_val = data.iter().copied().fold(f32::INFINITY, f32::min);
        let min_data_db = Decibels::from_linear(min_data_val).0;
        let max_data_val = data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let max_data_db = Decibels::from_linear(max_data_val).0;
        self.min_db = Some(min_data_db.min(self.min_db.unwrap_or(f32::INFINITY)));
        self.max_db = Some(max_data_db.max(self.max_db.unwrap_or(f32::NEG_INFINITY)));
    }

    fn insert_spectrum_line(&mut self, data: &[f32]) {
        let Some(fft_size) = self.engine_state.as_ref().map(|s| s.fft_size) else {
            return;
        };

        let new_pixels: Vec<Color32> = data
            .iter()
            .copied()
            .map(|m| self.magnitude_to_intensity(m))
            .map(|i| self.intensity_to_color(i))
            .collect();

        self.waterfall_texture.image.pixels.extend(new_pixels);
        self.waterfall_texture.image.pixels.rotate_right(data.len());
        self.waterfall_texture
            .image
            .pixels
            .truncate(fft_size * self.waterfall_max_lines);
        let img_height = self.waterfall_texture.image.pixels.len() / fft_size;
        self.waterfall_texture.image.size = [fft_size, img_height];

        // Mark that we have new pixel data to upload to GPU
        self.waterfall_texture.mark_updated();
    }

    fn magnitude_to_intensity(&self, magnitude: f32) -> f32 {
        // Convert linear magnitude to dB
        if magnitude <= 0.0 {
            return 0.0; // Avoid log(0)
        }

        let db = 20.0 * magnitude.log10();

        // Avoid division by zero if range is too small
        let range = self.max_db.unwrap() - self.min_db.unwrap();
        if range < 0.01 {
            return 0.5; // Middle gray if all values are the same
        }

        // Normalize to [0, 1] range
        ((db - self.min_db.unwrap()) / range).clamp(0.0, 1.0)
    }

    fn intensity_to_color(&self, intensity: f32) -> Color32 {
        // Simple grayscale mapping
        let value = (intensity * 255.0) as u8;
        Color32::from_gray(value)
    }
}
