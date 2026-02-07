use eframe::egui::{ColorImage, Image, Response, TextureHandle, TextureOptions, Ui, Widget};
use eframe::epaint::Color32;
use rustiq_messages::Decibels;

/// Waterfall display widget that renders a scrolling spectrogram.
///
/// This widget implements the egui `Widget` trait for `&mut Waterfall`, allowing it
/// to be used with `ui.add(&mut waterfall)`. It manages its own texture state and
/// handles GPU uploads efficiently.
///
/// The image pixels are updated via `insert_spectrum_line()` when new spectrum data
/// arrives. The `needs_gpu_upload` flag tracks whether the texture needs to be
/// re-uploaded to the GPU, avoiding redundant uploads when rendering multiple frames
/// without new data.
pub struct Waterfall {
    image: ColorImage,
    needs_gpu_upload: bool,
    /// Cached texture handle to avoid re-uploading on every frame
    waterfall_texture_handle: Option<TextureHandle>,

    // TODO: These could be monotonic stacks to keep track of the min/max value on screen instead of all time
    /// Min value in the waterfall. Used to scale the colors
    min_px_val: Option<Decibels>,
    /// Max value in the waterfall. Used to scale the colors
    max_px_val: Option<Decibels>,
}

impl Waterfall {
    pub fn new() -> Self {
        Self {
            image: ColorImage::default(),
            needs_gpu_upload: false,
            waterfall_texture_handle: None,
            min_px_val: None,
            max_px_val: None,
        }
    }

    /// Insert new line of pixel data at the top of the waterfall
    pub fn insert_spectrum_line(&mut self, data: &[f32]) {
        if data.is_empty() {
            return;
        };

        let img_width = data.len();
        if !self.image.pixels.is_empty() {
            assert_eq!(self.image.size[0], img_width);
        }

        let decibels: Vec<Decibels> = data.iter().map(|&f| Decibels::from_linear(f)).collect();
        self.update_min_max_values(&decibels);

        let new_pixels: Vec<Color32> = decibels
            .iter()
            .map(|&db| self.decibels_to_color(db))
            .collect();

        self.image.pixels.extend(new_pixels);
        self.image.pixels.rotate_right(img_width);

        assert_eq!(self.image.pixels.len() % img_width, 0);
        self.image.size = [img_width, self.image.pixels.len() / img_width];
        self.needs_gpu_upload = true;
    }

    fn decibels_to_color(&self, decibels: Decibels) -> Color32 {
        let min_val = self.min_px_val
            .expect("Tried to calculate a waterfall pixel color before establishing the min value to scale colors from");
        let max_val = self.max_px_val
            .expect("Tried to calculate a waterfall pixel color before establishing the max value to scale colors from");

        debug_assert!(decibels >= min_val);
        debug_assert!(decibels <= max_val);

        let range_len = max_val.0 - min_val.0;
        let scaled = (decibels.0 - min_val.0) / range_len.max(0.01); // avoid div by 0
        Color32::from_gray((scaled * 255.0) as u8)
    }

    fn update_min_max_values(&mut self, decibels: &[Decibels]) {
        assert!(!decibels.is_empty());
        let min_new = decibels.iter().min_by(|&a, &b| a.total_cmp(*b)).unwrap();
        let max_new = decibels.iter().max_by(|&a, &b| a.total_cmp(*b)).unwrap();

        let current_min = self.min_px_val.get_or_insert(Decibels(f32::INFINITY));
        if min_new < current_min {
            *current_min = *min_new;
        }

        let current_max = self.max_px_val.get_or_insert(Decibels(f32::NEG_INFINITY));
        if max_new > current_max {
            *current_max = *max_new;
        }
    }
}

impl Widget for &mut Waterfall {
    /// Renders the waterfall display.
    ///
    /// Pixel data is pre-computed in `insert_spectrum_line()` (not during rendering),
    /// so this function only uploads the texture to the GPU when new data is available.
    /// The texture handle is cached to avoid re-uploading on every frame.
    fn ui(self, ui: &mut Ui) -> Response {
        // Check if we have any image data
        if self.image.pixels.is_empty() {
            ui.label("Waiting for spectrum data...");
            return ui.response();
        }
        self.image.pixels.truncate(self.image.size.iter().product());

        // Only upload texture if we have new data
        if self.needs_gpu_upload {
            let texture =
                ui.ctx()
                    .load_texture("waterfall", self.image.clone(), TextureOptions::LINEAR);

            // Cache the texture handle for reuse
            self.waterfall_texture_handle = Some(texture);

            // Mark as uploaded to avoid redundant uploads on subsequent frames
            self.needs_gpu_upload = false;
        }

        // Display the cached texture (no clone or upload)
        if let Some(texture_handle) = &self.waterfall_texture_handle {
            let available_size = ui.available_size();
            // ui.add(eframe::egui::Image::new(texture_handle).fit_to_exact_size(available_size));
            ui.add(Image::new(texture_handle).fit_to_exact_size(available_size));
        }

        ui.response()
    }
}
