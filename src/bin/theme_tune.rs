use std::{
    sync::mpsc::{
        self, channel, Receiver, Sender,
        TryRecvError::{Disconnected, Empty},
    },
    thread,
};

use eframe::{egui, epaint::ColorImage};
use egui_extras::RetainedImage;
use image::{io::Reader, DynamicImage, Rgb};
use palette::{FromColor, Hsl, Srgb};
use wfinfo::{
    database::Database,
    ocr::{self, normalize_string},
    theme::{HslRange, Theme},
};

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Tune theme detection",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {
    original_image: DynamicImage,
    image: Option<RetainedImage>,

    ocr_request_sender: Sender<HslRange<f32>>,
    ocr_response_receiver: Receiver<Vec<(String, String)>>,
    ocr_result: Option<Vec<(String, String)>>,

    settings: HslRange<f32>,
}

impl Default for MyApp {
    fn default() -> Self {
        let original_image = Reader::open(std::env::args().nth(1).unwrap())
            .unwrap()
            .decode()
            .unwrap();
        let settings = HslRange {
            saturation: 0.50..1.0,
            lightness: 0.15..1.0,
            hue: -10.0..10.0,
        };
        let (ocr_request_sender, ocr_response_receiver) = spawn_ocr_thread(&original_image);
        Self {
            original_image,
            image: None,

            ocr_request_sender,
            ocr_response_receiver,
            ocr_result: None,

            settings,
        }
    }
}

fn spawn_ocr_thread(
    image: &DynamicImage,
) -> (Sender<HslRange<f32>>, Receiver<Vec<(String, String)>>) {
    let (request_sender, request_receiver): (Sender<_>, Receiver<_>) = channel();
    let (response_sender, response_receiver) = channel();
    let image = image.to_owned();

    thread::spawn(move || loop {
        let database = Database::load_from_file(None, None);
        loop {
            let mut last_request: HslRange<f32> = request_receiver.recv().unwrap();
            loop {
                match request_receiver.try_recv() {
                    Ok(request) => last_request = request,
                    Err(Empty) => break,
                    Err(Disconnected) => return,
                }
            }
            let strings = ocr::image_to_strings(
                image.clone(),
                Some(Theme::Custom(last_request.to_ordered())),
            );
            let results = strings
                .iter()
                .map(|string| {
                    let item = database.find_item(&normalize_string(string), None);
                    (
                        string.to_owned(),
                        item.map(|item| item.name.to_owned())
                            .unwrap_or("None".to_string()),
                    )
                })
                .collect();
            response_sender.send(results).unwrap();
        }
    });

    (request_sender, response_receiver)
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.image.is_none() {
            let image = self.process_image(&self.original_image);
            self.image = Some(convert_image(&image));
            self.ocr_request_sender.send(self.settings.clone()).unwrap();
        }

        match self.ocr_response_receiver.try_recv() {
            Ok(response) => self.ocr_result = Some(response),
            Err(Empty) => {}
            other => {
                other.unwrap();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.image.as_ref() {
            Some(image) => {
                image.show_scaled(ui, 3.0);
                if let Some(detections) = self.ocr_result.as_ref() {
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
                    egui::Slider::new(&mut self.settings.saturation.start, 0.0..=1.0)
                        .text("Saturation min"),
                )
                .changed()
                || ui
                    .add(
                        egui::Slider::new(&mut self.settings.saturation.end, 0.0..=1.0)
                            .text("Saturation max"),
                    )
                    .changed()
                || ui
                    .add(
                        egui::Slider::new(&mut self.settings.lightness.start, 0.0..=1.0)
                            .text("Lightness min"),
                    )
                    .changed()
                || ui
                    .add(
                        egui::Slider::new(&mut self.settings.lightness.end, 0.0..=1.0)
                            .text("Lightness max"),
                    )
                    .changed()
                || ui
                    .add(
                        egui::Slider::new(&mut self.settings.hue.start, -180.0..=180.0)
                            .text("Hue min"),
                    )
                    .changed()
                || ui
                    .add(
                        egui::Slider::new(&mut self.settings.hue.end, -180.0..=180.0)
                            .text("Hue max"),
                    )
                    .changed()
            {
                self.image = None;
                self.ocr_request_sender.send(self.settings.clone()).unwrap();
            };
        });
    }
}

impl MyApp {
    fn process_image(&self, image: &DynamicImage) -> DynamicImage {
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

        for pixel in new_image.pixels_mut() {
            let rgb = Srgb::from_components((
                pixel.0[0] as f32 / 255.0,
                pixel.0[1] as f32 / 255.0,
                pixel.0[2] as f32 / 255.0,
            ));
            let test = Hsl::from_color(rgb);

            let is_theme = self.settings.saturation.contains(&test.saturation)
                && self.settings.lightness.contains(&test.lightness)
                && self.settings.hue.contains(&test.hue.to_degrees());

            *pixel = if is_theme { Rgb([0; 3]) } else { Rgb([255; 3]) }
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
