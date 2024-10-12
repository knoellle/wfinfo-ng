use std::thread::sleep;
use std::time::Duration;
use std::{error::Error, str::FromStr};
use std::{fs::File, thread};
use std::{
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    sync::mpsc::channel,
};
use std::{path::PathBuf, sync::mpsc};

use clap::{Parser};
use env_logger::{Builder, Env};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use log::{debug, error, info, warn};
use notify::{watcher, RecursiveMode, Watcher};
use xcap::{Window, Monitor};
use std::time::{SystemTime, UNIX_EPOCH};
use bytemuck::cast_slice;

use wfinfo::{
    database::Database,
    ocr::{normalize_string, reward_image_to_reward_names, OCR},
};

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage,  Luma};


fn save_debug_image(image: &DynamicImage, filename: &str) -> Result<(), Box<dyn Error>> {
    image.save_with_format(filename, ImageFormat::Png)?;
    info!("Saved debug image: {}", filename);
    Ok(())
}

fn hdr_to_sdr(image: &ImageBuffer<Rgba<f32>, Vec<f32>>, luminescence: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut sdr_image = ImageBuffer::new(image.width(), image.height());

    // Normalize luminescence to 0-1 range
    let max_luminescence = 1000.0;
    let lum_factor = (luminescence as f32 / max_luminescence).min(1.0).max(0.1);

    // Calculate the average luminance of the image
    let mut max_luminance = 0.0f32;
    for pixel in image.pixels() {
        let luminance = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
        max_luminance = max_luminance.max(luminance);
    }

    // Adjust max_luminance based on the luminescence factor
    max_luminance *= lum_factor;

    for (x, y, pixel) in image.enumerate_pixels() {
        let r = pixel[0];
        let g = pixel[1];
        let b = pixel[2];

        // Apply tone mapping (Reinhard operator)
        let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let scaled_luminance = luminance / max_luminance;
        let mapped_luminance = scaled_luminance / (1.0 + scaled_luminance);

        // Apply color correction
        let scale = mapped_luminance / luminance;
        let r_sdr = (r * scale * 255.0).min(255.0) as u8;
        let g_sdr = (g * scale * 255.0).min(255.0) as u8;
        let b_sdr = (b * scale * 255.0).min(255.0) as u8;

        // Apply gamma correction
        let gamma = 1.0 / 2.2;
        let r_gamma = ((r_sdr as f32 / 255.0).powf(gamma) * 255.0) as u8;
        let g_gamma = ((g_sdr as f32 / 255.0).powf(gamma) * 255.0) as u8;
        let b_gamma = ((b_sdr as f32 / 255.0).powf(gamma) * 255.0) as u8;

        sdr_image.put_pixel(x, y, Rgba([r_gamma, g_gamma, b_gamma, (pixel[3] * 255.0) as u8]));
    }

    sdr_image
}

fn preprocess_for_ocr(image: &DynamicImage) -> DynamicImage {
    let gray_image = image.to_luma8();

    // Apply adaptive thresholding
    let threshold_image = adaptive_threshold(&gray_image, 11, 2);

    DynamicImage::ImageLuma8(threshold_image)
}

fn adaptive_threshold(image: &ImageBuffer<Luma<u8>, Vec<u8>>, block_size: u32, c: i32) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let mut output = ImageBuffer::new(image.width(), image.height());
    let half_block = block_size / 2;

    for (x, y, pixel) in image.enumerate_pixels() {
        let mut sum = 0u32;
        let mut count = 0u32;

        for i in x.saturating_sub(half_block)..=(x + half_block).min(image.width() - 1) {
            for j in y.saturating_sub(half_block)..=(y + half_block).min(image.height() - 1) {
                sum += image.get_pixel(i, j).0[0] as u32;
                count += 1;
            }
        }

        let threshold = (sum / count) as i32 - c;
        let new_value = if pixel.0[0] as i32 > threshold { 255 } else { 0 };
        output.put_pixel(x, y, Luma([new_value]));
    }

    output
}




fn run_detection(capturer: &dyn Capturable, db: &Database, is_hdr: bool, luminescence: u32, save_debug_images: bool) {
    let frame = capturer.capture_image().unwrap();
    info!("Captured");

    let image = if is_hdr {
        info!("Converting HDR to SDR");
        let converted = DynamicImage::ImageRgba8(hdr_to_sdr(&frame, luminescence));

        if save_debug_images {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Save the original HDR frame
            if let Err(e) = image::save_buffer_with_format(
                format!("debug_original_{}.exr", timestamp),
                cast_slice(frame.as_raw()),
                frame.width(),
                frame.height(),
                image::ColorType::Rgba32F,
                image::ImageFormat::OpenExr,
            ) {
                warn!("Failed to save original debug image: {}", e);
            }

            // Save the converted SDR image
            if let Err(e) = save_debug_image(&converted, &format!("debug_converted_{}.png", timestamp)) {
                warn!("Failed to save converted debug image: {}", e);
            }
        }

        // Apply preprocessing only for HDR images
        let preprocessed = preprocess_for_ocr(&converted);

        if save_debug_images {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if let Err(e) = save_debug_image(&preprocessed, &format!("debug_preprocessed_{}.png", timestamp)) {
                warn!("Failed to save preprocessed debug image: {}", e);
            }
        }

        preprocessed
    } else {
        DynamicImage::ImageRgba8(rgbaf32_to_rgba8(&frame))
    };
    info!("Image prepared for OCR");

    fn rgbaf32_to_rgba8(rgba: &ImageBuffer<Rgba<f32>, Vec<f32>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = rgba.dimensions();
    let mut u8_buffer = Vec::with_capacity((width * height * 4) as usize);

    for pixel in rgba.pixels() {
        u8_buffer.push((pixel[0] * 255.0) as u8);
        u8_buffer.push((pixel[1] * 255.0) as u8);
        u8_buffer.push((pixel[2] * 255.0) as u8);
        u8_buffer.push((pixel[3] * 255.0) as u8);
    }

    ImageBuffer::from_raw(width, height, u8_buffer).unwrap()
}



    let text = reward_image_to_reward_names(image, None);
    let text = text.iter().map(|s| normalize_string(s));
    debug!("{:#?}", text);

    let items: Vec<_> = text.map(|s| db.find_item(&s, None)).collect();

    let best = items
        .iter()
        .map(|item| {
            item.map(|item| {
                item.platinum
                    .max(item.ducats as f32 / 10.0 + item.platinum / 100.0)
            })
            .unwrap_or(0.0)
        })
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|best| best.0);

    for (index, item) in items.iter().enumerate() {
        if let Some(item) = item {
            info!(
                "{}\n\t{}\t{}\t{}",
                item.drop_name,
                item.platinum,
                item.ducats as f32 / 10.0,
                if Some(index) == best { "<----" } else { "" }
            );
        } else {
            warn!("Unknown item\n\tUnknown");
        }
    }
}

fn log_watcher(path: PathBuf, event_sender: mpsc::Sender<()>) {
    debug!("Path: {}", path.display());
    let mut position = File::open(&path)
        .unwrap_or_else(|_| panic!("Couldn't open file {}", path.display()))
        .seek(SeekFrom::End(0))
        .unwrap();

    thread::spawn(move || {
        debug!("Position: {}", position);

        let (tx, rx) = mpsc::channel();
        let mut watcher = watcher(tx, Duration::from_millis(100)).unwrap();
        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .unwrap_or_else(|_| panic!("Failed to open EE.log file: {}", path.display()));

        loop {
            match rx.recv() {
                Ok(notify::DebouncedEvent::Write(_)) => {
                    let mut f = File::open(&path).unwrap();
                    f.seek(SeekFrom::Start(position)).unwrap();

                    let mut reward_screen_detected = false;

                    let reader = BufReader::new(f.by_ref());
                    for line in reader.lines() {
                        let line = match line {
                            Ok(line) => line,
                            Err(err) => {
                                error!("Error reading line: {}", err);
                                continue;
                            }
                        };
                        // debug!("> {:?}", line);
                        if line.contains("Pause countdown done")
                            || line.contains("Got rewards")
                            || line.contains("Created /Lotus/Interface/ProjectionRewardChoice.swf")
                        {
                            reward_screen_detected = true;
                        }
                    }

                    if reward_screen_detected {
                        info!("Detected, waiting...");
                        sleep(Duration::from_millis(1500));
                        event_sender.send(()).unwrap();
                    }

                    position = f.metadata().unwrap().len();
                    debug!("Log position: {}", position);
                }
                Ok(_) => {}
                Err(err) => {
                    error!("Error: {:?}", err);
                }
            }
        }
    });
}

fn hotkey_watcher(hotkey: HotKey, event_sender: mpsc::Sender<()>) {
    debug!("watching hotkey: {hotkey:?}");
    thread::spawn(move || {
        let manager = GlobalHotKeyManager::new().unwrap();
        manager.register(hotkey).unwrap();

        while let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
            debug!("{:?}", event);
            if event.state == HotKeyState::Pressed {
                event_sender.send(()).unwrap();
            }
        }
    });
}

#[allow(dead_code)]
fn benchmark() -> Result<(), Box<dyn Error>> {
    for _ in 0..10 {
        let image = image::open("input3.png").unwrap();
        println!("Converted");
        let text = reward_image_to_reward_names(image, None);
        println!("got names");
        let text = text.iter().map(|s| normalize_string(s));
        println!("{:#?}", text);
    }
    // clean up tesseract
    drop(OCR.lock().unwrap().take());
    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Path to the `EE.log` file located in the game installation directory
    ///
    /// Most likely located at `~/.local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log`
    game_log_file_path: Option<PathBuf>,
    /// Warframe Window Name
    ///
    /// some systems may require the window name to be specified (e.g. when using gamescope)
    #[arg(short, long, default_value = "Warframe")]
    window_name: String,
    /// Monitor number to capture (0-based index)
    ///
    /// If specified, this will be used instead of the window name
    #[arg(short, long)]
    monitor: Option<usize>,
    /// Specify if the monitor is in HDR mode
    #[arg(long)]
    hdr: bool,
    /// Luminescence level for HDR (100-1000 nits)
    #[arg(long, default_value = "300")]
    luminescence: u32,
    /// Save debug images when HDR conversion is applied
    #[arg(long)]
    save_debug_images: bool,
}

trait Capturable {
    fn capture_image(&self) -> Result<image::ImageBuffer<Rgba<f32>, Vec<f32>>, Box<dyn Error>>;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
}


impl Capturable for Window {
    fn capture_image(&self) -> Result<ImageBuffer<Rgba<f32>, Vec<f32>>, Box<dyn Error>> {
        let rgba_image: RgbaImage = self.capture_image()?;
        let float_image = rgba_to_rgbaf32(&rgba_image);
        Ok(float_image)
    }

    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
    }
}

impl Capturable for Monitor {
    fn capture_image(&self) -> Result<ImageBuffer<Rgba<f32>, Vec<f32>>, Box<dyn Error>> {
        let rgba_image: RgbaImage = self.capture_image()?;
        let float_image = rgba_to_rgbaf32(&rgba_image);
        Ok(float_image)
    }

    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
    }
}

fn rgba_to_rgbaf32(rgba: &RgbaImage) -> ImageBuffer<Rgba<f32>, Vec<f32>> {
    let (width, height) = rgba.dimensions();
    let mut float_buffer = Vec::with_capacity((width * height * 4) as usize);

    for pixel in rgba.pixels() {
        float_buffer.push(pixel[0] as f32 / 255.0);
        float_buffer.push(pixel[1] as f32 / 255.0);
        float_buffer.push(pixel[2] as f32 / 255.0);
        float_buffer.push(pixel[3] as f32 / 255.0);
    }

    ImageBuffer::from_raw(width, height, float_buffer).unwrap()
}


fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();
    let default_log_path = PathBuf::from_str(&std::env::var("HOME").unwrap()).unwrap().join(PathBuf::from_str(".local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log")?);
    let log_path = arguments.game_log_file_path.unwrap_or(default_log_path);
    let env = Env::default()
        .filter_or("WFINFO_LOG", "info")
        .write_style_or("WFINFO_STYLE", "always");
    Builder::from_env(env)
        .format_timestamp(None)
        .format_level(false)
        .format_module_path(false)
        .format_target(false)
        .init();

    let db = Database::load_from_file(None, None);

    let capturer: Box<dyn Capturable> = if let Some(monitor_index) = arguments.monitor {
        let monitors = Monitor::all()?;
        if monitor_index >= monitors.len() {
            return Err(format!("Invalid monitor index: {}", monitor_index).into());
        }
        Box::new(monitors[monitor_index].clone())
    } else {
        let windows = Window::all()?;
        let Some(warframe_window) = windows.iter().find(|x| x.title() == arguments.window_name) else {
            return Err("Warframe window not found".into());
        };
        Box::new(warframe_window.clone())
    };

    debug!(
        "Capture source resolution: {:?}x{:?}",
        capturer.width(),
        capturer.height()
    );

    info!("Loaded database");

    let (event_sender, event_receiver) = channel();

    log_watcher(log_path, event_sender.clone());
    hotkey_watcher("F12".parse()?, event_sender);

    while let Ok(()) = event_receiver.recv() {
        info!("Capturing");
        run_detection(&*capturer, &db, arguments.hdr, arguments.luminescence, arguments.save_debug_images);
    }

    drop(OCR.lock().unwrap().take());
    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;
    use std::fs::read_to_string;

    use image::io::Reader;
    use indexmap::IndexMap;
    use rayon::prelude::*;
    use tesseract::Tesseract;
    use wfinfo::ocr::detect_theme;
    use wfinfo::ocr::extract_parts;
    use wfinfo::testing::Label;

    use super::*;

    #[test]
    fn single_image() {
        let image = Reader::open(format!("test-images/{}.png", 1))
            .unwrap()
            .decode()
            .unwrap();
        let text = reward_image_to_reward_names(image, None);
        let text = text.iter().map(|s| normalize_string(s));
        println!("{:#?}", text);
        let db = Database::load_from_file(None, None);
        let items: Vec<_> = text.map(|s| db.find_item(&s, None)).collect();
        println!("{:#?}", items);

        assert_eq!(
            items[0].expect("Didn't find an item?").drop_name,
            "Octavia Prime Systems Blueprint"
        );
        assert_eq!(
            items[1].expect("Didn't find an item?").drop_name,
            "Octavia Prime Blueprint"
        );
        assert_eq!(
            items[2].expect("Didn't find an item?").drop_name,
            "Tenora Prime Blueprint"
        );
        assert_eq!(
            items[3].expect("Didn't find an item?").drop_name,
            "Harrow Prime Systems Blueprint"
        );
    }

    // #[test]
    #[allow(dead_code)]
    fn wfi_images_exact() {
        let labels: IndexMap<String, Label> =
            serde_json::from_str(&read_to_string("WFI test images/labels.json").unwrap()).unwrap();
        for (filename, label) in labels {
            let image = Reader::open("WFI test images/".to_string() + &filename)
                .unwrap()
                .decode()
                .unwrap();
            let text = reward_image_to_reward_names(image, None);
            let text: Vec<_> = text.iter().map(|s| normalize_string(s)).collect();
            println!("{:#?}", text);

            let db = Database::load_from_file(None, None);
            let items: Vec<_> = text.iter().map(|s| db.find_item(s, None)).collect();
            println!("{:#?}", items);
            println!("{}", filename);

            let item_names = items
                .iter()
                .map(|item| item.map(|item| item.drop_name.clone()));

            for (result, expectation) in item_names.zip(label.items) {
                if expectation.is_empty() {
                    assert_eq!(result, None)
                } else {
                    assert_eq!(result, Some(expectation))
                }
            }
        }
    }

    #[test]
    fn wfi_images_99_percent() {
        let labels: BTreeMap<String, Label> =
            serde_json::from_str(&read_to_string("WFI test images/labels.json").unwrap()).unwrap();
        let total = labels.len();
        let success_count: usize = labels
            .into_par_iter()
            .map(|(filename, label)| {
                let image = Reader::open("WFI test images/".to_string() + &filename)
                    .unwrap()
                    .decode()
                    .unwrap();
                let text = reward_image_to_reward_names(image, None);
                let text: Vec<_> = text.iter().map(|s| normalize_string(s)).collect();
                println!("{:#?}", text);

                let db = Database::load_from_file(None, None);
                let items: Vec<_> = text.iter().map(|s| db.find_item(s, None)).collect();
                println!("{:#?}", items);
                println!("{}", filename);

                let item_names = items
                    .iter()
                    .map(|item| item.map(|item| item.drop_name.clone()));

                if item_names.zip(label.items).all(|(result, expectation)| {
                    expectation == result.unwrap_or_else(|| "".to_string())
                }) {
                    1
                } else {
                    0
                }
            })
            .sum();

        let success_rate = success_count as f32 / total as f32;
        assert!(success_rate > 0.95, "Success rate: {success_rate}");
    }

    // #[test]
    #[allow(dead_code)]
    fn images() {
        let tests = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
        for i in tests {
            let image = Reader::open(format!("test-images/{}.png", i))
                .unwrap()
                .decode()
                .unwrap();

            let theme = detect_theme(&image);
            println!("Theme: {:?}", theme);

            let parts = extract_parts(&image, theme);

            let mut ocr =
                Tesseract::new(None, Some("eng")).expect("Could not initialize Tesseract");
            for part in parts {
                let buffer = part.as_flat_samples_u8().unwrap();
                ocr = ocr
                    .set_frame(
                        buffer.samples,
                        part.width() as i32,
                        part.height() as i32,
                        3,
                        3 * part.width() as i32,
                    )
                    .expect("Failed to set image");
                let text = ocr.get_text().expect("Failed to get text");
                println!("{}", text);
            }
            println!("=================");
        }
    }
}
