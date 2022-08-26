use approx::AbsDiffEq;
use eframe::{egui, epaint::ColorImage};
use egui_extras::RetainedImage;
use image::{io::Reader, DynamicImage, Rgb};
use palette::{FromColor, Hsl, Srgb};
use wfinfo::theme::{color_difference, Theme};

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Tune theme detection",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {
    original_image: Option<DynamicImage>,
    image: Option<RetainedImage>,
    saturation: f32,
    hue: f32,
}

impl Default for MyApp {
    fn default() -> Self {
        let original_image = Some(
            Reader::open(std::env::args().nth(1).unwrap())
                .unwrap()
                .decode()
                .unwrap(),
        );
        Self {
            original_image,
            image: None,

            saturation: 0.55,
            hue: 0.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(original_image) = self.original_image.as_ref() {
            if self.image.is_none() {
                self.image = Some(convert_image(&self.process_image(original_image)));
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.image.as_ref() {
                Some(image) => {
                    image.show_max_size(ui, ui.available_size());
                }
                None => {
                    ui.spinner(); // still loading
                }
            }
        });
        egui::TopBottomPanel::bottom("Bottom Panel").show(ctx, |ui| {
            if ui
                .add(egui::Slider::new(&mut self.saturation, 0.0..=1.0).text("Saturation"))
                .changed()
            {
                self.image = None
            };
            if ui
                .add(egui::Slider::new(&mut self.hue, 0.0..=360.0).text("Hue"))
                .changed()
            {
                self.image = None
            };
        });
    }
}

impl MyApp {
    fn process_image(&self, image: &DynamicImage) -> DynamicImage {
        let mut new_image = image.to_rgb8();

        let primary = Theme::Stalker.primary();
        let secondary = Theme::Stalker.secondary();

        for pixel in new_image.pixels_mut() {
            let rgb = Srgb::from_components((
                pixel.0[0] as f32 / 255.0,
                pixel.0[1] as f32 / 255.0,
                pixel.0[2] as f32 / 255.0,
            ));
            let test = Hsl::from_color(rgb);

            let is_theme = test.saturation > self.saturation
                && test.hue.to_degrees().abs_diff_eq(&self.hue, 4.0);
            let is_theme = color_difference((test, primary)) < self.saturation * 255.0;

            *pixel = if is_theme { Rgb([255; 3]) } else { Rgb([0; 3]) }
        }
        DynamicImage::ImageRgb8(new_image)
    }
}

fn convert_image(original_image: &DynamicImage) -> RetainedImage {
    let ui_image = ColorImage::from_rgba_unmultiplied(
        [original_image.width() as _, original_image.height() as _],
        &original_image.to_rgba8(),
    );
    RetainedImage::from_color_image("Temp", ui_image)
        .with_texture_filter(egui::TextureFilter::Nearest)
}
