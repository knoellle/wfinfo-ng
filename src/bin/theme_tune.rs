use std::ops::Range;

use eframe::{egui, epaint::ColorImage};
use egui_extras::RetainedImage;
use image::{io::Reader, DynamicImage, Rgb};
use palette::{FromColor, Hsl, Srgb};
use wfinfo::theme::Theme;

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
    detections: Option<Vec<String>>,

    saturation: Range<f32>,
    lightness: Range<f32>,
    hue: Range<f32>,
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
            detections: None,

            saturation: 0.55..1.0,
            lightness: 0.0..1.0,
            hue: 0.0..5.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(original_image) = self.original_image.as_ref() {
            if self.image.is_none() {
                let (image, detections) = self.process_image(original_image);
                self.image = Some(convert_image(&image));
                self.detections = Some(detections);
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.image.as_ref() {
            Some(image) => {
                image.show_scaled(ui, 3.0);
                if let Some(detections) = self.detections.as_ref() {
                    ui.label(format!("{:#?}", detections));
                }
            }
            None => {
                ui.spinner();
            }
        });
        egui::TopBottomPanel::bottom("Bottom Panel").show(ctx, |ui| {
            if ui
                .add(
                    egui::Slider::new(&mut self.saturation.start, 0.0..=1.0).text("Saturation min"),
                )
                .changed()
                || ui
                    .add(
                        egui::Slider::new(&mut self.saturation.end, 0.0..=1.0)
                            .text("Saturation max"),
                    )
                    .changed()
                || ui
                    .add(
                        egui::Slider::new(&mut self.lightness.start, 0.0..=1.0)
                            .text("Lightness min"),
                    )
                    .changed()
                || ui
                    .add(
                        egui::Slider::new(&mut self.lightness.end, 0.0..=1.0).text("Lightness max"),
                    )
                    .changed()
                || ui
                    .add(egui::Slider::new(&mut self.hue.start, -180.0..=180.0).text("Hue min"))
                    .changed()
                || ui
                    .add(egui::Slider::new(&mut self.hue.end, -180.0..=180.0).text("Hue max"))
                    .changed()
            {
                self.image = None
            };
        });
    }
}

impl MyApp {
    fn process_image(&self, image: &DynamicImage) -> (DynamicImage, Vec<String>) {
        const PIXEL_REWARD_WIDTH: f32 = 968.0;
        const PIXEL_REWARD_HEIGHT: f32 = 235.0;
        const PIXEL_REWARD_YDISPLAY: f32 = 316.0;
        const PIXEL_REWARD_LINE_HEIGHT: f32 = 48.0;

        let screen_scaling = if image.width() * 9 > image.height() * 16 {
            image.height() as f32 / 1080.0
        } else {
            image.width() as f32 / 1920.0
        };

        let width = image.width() as f32;
        let height = image.height() as f32;
        let most_width = PIXEL_REWARD_WIDTH * screen_scaling;
        let most_left = width / 2.0 - most_width / 2.0;
        // Most Top = pixleRewardYDisplay - pixleRewardHeight + pixelRewardLineHeight
        //                   (316          -        235        +       44)    *    1.1    =    137
        let most_top = height / 2.0
            - ((PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT + PIXEL_REWARD_LINE_HEIGHT)
                * screen_scaling);
        let most_bot = height / 2.0
            - ((PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT) as f32 * screen_scaling * 0.5);

        let mut new_image = image
            .crop_imm(
                most_left as u32,
                most_top as u32,
                most_width as u32,
                (most_bot - most_top) as u32,
            )
            .to_rgb8();

        let _primary = Theme::Stalker.primary();
        let _secondary = Theme::Stalker.secondary();

        for pixel in new_image.pixels_mut() {
            let rgb = Srgb::from_components((
                pixel.0[0] as f32 / 255.0,
                pixel.0[1] as f32 / 255.0,
                pixel.0[2] as f32 / 255.0,
            ));
            let test = Hsl::from_color(rgb);

            let is_theme = self.saturation.contains(&test.saturation)
                && self.lightness.contains(&test.lightness)
                && self.hue.contains(&test.hue.to_degrees());
            // let is_theme = color_difference((test, primary)) < self.hue
            //     || color_difference((test, secondary)) < self.hue;

            *pixel = if is_theme { Rgb([0; 3]) } else { Rgb([255; 3]) }
        }

        let detections = ocr::image_to_strings(image.to_owned());

        (DynamicImage::ImageRgb8(new_image), detections)
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
