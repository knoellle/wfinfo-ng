use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::{error::Error, str::FromStr};
use std::{fs::File, thread};
use std::{
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    sync::mpsc::channel,
};
use std::{path::PathBuf, sync::mpsc};

use clap::Parser;
use env_logger::{Builder, Env};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use image::DynamicImage;
use log::{debug, error, info, warn};
use notify::{watcher, RecursiveMode, Watcher};
use xcap::Window;

use wfinfo::{
    config::{BestItemMode, InfoDisplayMode},
    database::Database,
    ocr::{
        extract_part, normalize_string, reward_image_to_reward_names, selection_to_part_name,
        slop_to_selection, SelectionParams, OCR,
    },
    utils::fetch_prices_and_items,
};

fn run_detection(capturer: &Window, db: &Database, arguments: &Arguments) {
    let frame = capturer.capture_image().unwrap();
    info!("Captured");
    let image = DynamicImage::ImageRgba8(frame);
    info!("Converted");
    let text = reward_image_to_reward_names(image, None);
    let text = text.iter().map(|s| normalize_string(s));
    debug!("{:#?}", text);

    let items: Vec<_> = text.map(|s| db.find_item(&s, None)).collect();

    let best = items
        .iter()
        .map(|item| {
            item.map(|item| match arguments.best_item_mode {
                BestItemMode::Combined => item
                    .platinum
                    .max(item.ducats as f32 / 10.0 + item.platinum / 100.0),
                BestItemMode::Platinum => item.platinum,
                BestItemMode::Ducats => item.ducats as f32 / 10.0,
                BestItemMode::Volatility => {
                    // Calculate sales volume: max(volume_yesterday - volume_today, 0)
                    let sales_volume = (item.yesterday_vol.saturating_sub(item.today_vol)) as f32;
                    // Calculate volatility: sales_volume * platinum
                    sales_volume * item.platinum
                }
            })
            .unwrap_or(0.0)
        })
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|best| best.0);

    for (index, item) in items.iter().enumerate() {
        if let Some(item) = item {
            match arguments.info_display_mode {
                InfoDisplayMode::Minimal => info!(
                    "{}\n\t{}\t{}\t{}",
                    item.drop_name,
                    item.platinum,
                    item.ducats as f32 / 10.0,
                    if Some(index) == best { "<----" } else { "" }
                ),
                InfoDisplayMode::Combined => info!(
                    "{}\n\tPlatinum: {}\tDucats: {}\t{}",
                    item.drop_name,
                    item.platinum,
                    item.ducats as f32 / 10.0,
                    if Some(index) == best { "<----" } else { "" }
                ),
                InfoDisplayMode::All => info!(
                    "{}\n\tPlatinum: {}\tDucats: {}\tYesterday Vol: {}\tToday Vol: {}\t{}",
                    item.drop_name,
                    item.platinum,
                    item.ducats as f32 / 10.0,
                    item.yesterday_vol,
                    item.today_vol,
                    if Some(index) == best { "<----" } else { "" }
                ),
            }
        } else {
            warn!("Unknown item\n\tUnknown");
        }
    }
}

fn run_snapit(window: &Window, db: &Database, arguments: &Arguments) -> Option<String> {
    // Capture the window
    let frame = window.capture_image().ok()?;
    let image = DynamicImage::ImageRgba8(frame);
    debug!("Captured window image");

    // Run slop to get the selection
    let slop_output = Command::new("slop")
        .args(["-b", "3", "-c", "1,0,0,0.8"])
        .output()
        .ok()?;
    let slop_output = String::from_utf8_lossy(&slop_output.stdout);
    debug!("Slop output: {}", slop_output);

    // Parse the selection coordinates
    let selection = slop_to_selection(&slop_output)?;
    debug!(
        "Selection: {}x{} at {},{}",
        selection.width, selection.height, selection.x, selection.y
    );

    // Get window position
    let window_x = window.x();
    let window_y = window.y();

    let cropped = extract_part(
        &image,
        (selection.width, selection.height),
        (selection.x - window_x, selection.y - window_y),
        arguments.ocr_brightness,
        arguments.ocr_contrast,
    );

    // Convert the selection to a part name
    let params = SelectionParams {
        abs_x: selection.x,
        abs_y: selection.y,
        width: selection.width,
        height: selection.height,
        monitor_x: window_x,
        monitor_y: window_y,
        brightness: arguments.ocr_brightness,
        contrast: arguments.ocr_contrast,
    };

    let text = selection_to_part_name(image.clone(), params)?;

    // Look up the item in the database
    let item = db.find_item(&normalize_string(&text), None);
    if let Some(item) = item {
        match arguments.info_display_mode {
            InfoDisplayMode::Minimal => {
                let volatility = (item.yesterday_vol.saturating_sub(item.today_vol)) as f32 * item.platinum;
                let (plat_fmt, ducat_fmt) = match arguments.best_item_mode {
                    BestItemMode::Platinum => ("\x1b[1;32m", "\x1b[0m"),
                    BestItemMode::Ducats => ("\x1b[0m", "\x1b[1;32m"),
                    BestItemMode::Combined => {
                        if item.platinum > item.ducats as f32 / 10.0 {
                            ("\x1b[1;32m", "\x1b[0m")
                        } else {
                            ("\x1b[0m", "\x1b[1;32m")
                        }
                    }
                    BestItemMode::Volatility => ("\x1b[0m", "\x1b[0m"),
                };
                info!(
                    "{}:\n\t{}{}p\x1b[0m, {}{}d\x1b[0m{}",
                    item.drop_name,
                    plat_fmt, item.platinum,
                    ducat_fmt, item.ducats as f32 / 10.0,
                    if arguments.best_item_mode == BestItemMode::Volatility {
                        format!("\n\tVolatility: \x1b[1;32m{}\x1b[0m", volatility)
                    } else {
                        String::new()
                    }
                );
            }
            InfoDisplayMode::Combined => {
                let volatility = (item.yesterday_vol.saturating_sub(item.today_vol)) as f32 * item.platinum;
                let (plat_fmt, ducat_fmt) = match arguments.best_item_mode {
                    BestItemMode::Platinum => ("\x1b[1;32m", "\x1b[0m"),
                    BestItemMode::Ducats => ("\x1b[0m", "\x1b[1;32m"),
                    BestItemMode::Combined => {
                        if item.platinum > item.ducats as f32 / 10.0 {
                            ("\x1b[1;32m", "\x1b[0m")
                        } else {
                            ("\x1b[0m", "\x1b[1;32m")
                        }
                    }
                    BestItemMode::Volatility => ("\x1b[0m", "\x1b[0m"),
                };
                info!(
                    "{}:\n\tPlatinum: {}{}p\x1b[0m\tDucats: {}{}d\x1b[0m{}",
                    item.drop_name,
                    plat_fmt, item.platinum,
                    ducat_fmt, item.ducats as f32 / 10.0,
                    if arguments.best_item_mode == BestItemMode::Volatility {
                        format!("\n\tVolatility: \x1b[1;32m{}\x1b[0m", volatility)
                    } else {
                        String::new()
                    }
                );
            }
            InfoDisplayMode::All => {
                let volatility = (item.yesterday_vol.saturating_sub(item.today_vol)) as f32 * item.platinum;
                let (plat_fmt, ducat_fmt) = match arguments.best_item_mode {
                    BestItemMode::Platinum => ("\x1b[1;32m", "\x1b[0m"),
                    BestItemMode::Ducats => ("\x1b[0m", "\x1b[1;32m"),
                    BestItemMode::Combined => {
                        if item.platinum > item.ducats as f32 / 10.0 {
                            ("\x1b[1;32m", "\x1b[0m")
                        } else {
                            ("\x1b[0m", "\x1b[1;32m")
                        }
                    }
                    BestItemMode::Volatility => ("\x1b[0m", "\x1b[0m"),
                };
                info!(
                    "{}:\n\tPlatinum: {}{}p\x1b[0m\tDucats: {}{}d\x1b[0m\tYesterday Vol: {}\tToday Vol: {}\t{}",
                    item.drop_name,
                    plat_fmt, item.platinum,
                    ducat_fmt, item.ducats as f32 / 10.0,
                    item.yesterday_vol,
                    item.today_vol,
                    if arguments.best_item_mode == BestItemMode::Volatility {
                        format!("\n\tVolatility: \x1b[1;32m{}\x1b[0m", volatility)
                    } else {
                        String::new()
                    }
                );
            }
        }
    } else {
        info!("No item found");
    }

    Some(text)
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Path to the `EE.log` file located in the game installation directory
    ///
    /// Most likely located at `~/.local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log`
    game_log_file_path: Option<PathBuf>,
    /// Warframe Window Name
    ///
    /// some systems may require the window name to be specified (e.g. when using gamescope)
    #[arg(short, long, default_value = "Warframe")]
    pub window_name: String,

    /// Skip window confirmation (debug only)
    #[arg(long)]
    pub skip_window_confirmation: bool,

    /// Best item mode
    ///
    /// - `combined`: Platinum + Ducats (Platinum / 100 + Ducats / 10)
    /// - `platinum`: Platinum
    /// - `ducats`: Ducats
    /// - `volatility`: Volatility (Max(volume_yesterday - volume_today, 0) * Platinum)
    #[arg(short, long, default_value = "combined")]
    #[clap(verbatim_doc_comment)]
    pub best_item_mode: BestItemMode,

    /// Info display mode
    ///
    /// - `minimal`: Minimal (Shows only the name, platinum, and ducats)
    /// - `combined`: Combined (Shows platinum and ducats, with labels)
    /// - `all`: All (Also shows today and yesterday's volumes)
    #[arg(short, long, default_value = "minimal")]
    #[clap(verbatim_doc_comment)]
    pub info_display_mode: InfoDisplayMode,

    /// Forma platinum multiplier
    ///
    /// The multiplier to use for Forma's platinum value
    #[arg(short, long, default_value = "1.0")]
    pub forma_platinum_multiplier: f32,

    /// Forma platinum value
    ///
    /// The base platinum value for Forma
    #[arg(short = 'v', long, default_value = "11.666667")] // 35.0/3.0
    pub forma_platinum_value: f32,

    /// Detection hotkey
    ///
    /// The hotkey to use for detection
    #[arg(short, long, default_value = "F12")]
    pub detection_hotkey: HotKey,

    /// Snap-it hotkey
    ///
    /// The hotkey to use for the snap-it feature
    #[arg(short, long, default_value = "F10")]
    pub snapit_hotkey: HotKey,

    /// Default X11 display
    #[arg(short, long)]
    pub x11_display: Option<String>,

    /// Default Wayland display
    #[arg(short = 'y', long)]
    pub wayland_display: Option<String>,

    /// Sleep duration between checks in milliseconds
    ///
    /// Controls how often to check for hotkey events. Lower values increase responsiveness but use more CPU.
    #[arg(long, default_value = "10")]
    pub sleep_duration: u64,

    /// OCR brightness adjustment (-255 to 255)
    #[arg(long, default_value = "30")]
    pub ocr_brightness: i32,

    /// OCR contrast adjustment (0.0 to 10.0)
    #[arg(long, default_value = "2.0")]
    pub ocr_contrast: f32,
}

fn setup_hotkeys(
    detection_hotkey: HotKey,
    snapit_hotkey: HotKey,
    detection_sender: mpsc::Sender<()>,
    snapit_sender: mpsc::Sender<()>,
) -> Result<GlobalHotKeyManager, Box<dyn Error>> {
    let hotkey_manager = GlobalHotKeyManager::new()?;
    debug!(
        "Registering hotkeys - F12: {}, F10: {}",
        detection_hotkey.id(),
        snapit_hotkey.id()
    );

    hotkey_manager.register(detection_hotkey)?;
    hotkey_manager.register(snapit_hotkey)?;

    let detection_id = detection_hotkey.id();
    let snapit_id = snapit_hotkey.id();

    // Single thread for handling both hotkeys
    let receiver = GlobalHotKeyEvent::receiver();
    thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            if event.state == HotKeyState::Pressed {
                match event.id {
                    id if id == detection_id => {
                        debug!("Detection hotkey pressed");
                        if let Err(e) = detection_sender.send(()) {
                            error!("Failed to send detection event: {}", e);
                        }
                    }
                    id if id == snapit_id => {
                        debug!("Snapit hotkey pressed");
                        if let Err(e) = snapit_sender.send(()) {
                            error!("Failed to send snapit event: {}", e);
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    Ok(hotkey_manager)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();
    let default_log_path = PathBuf::from_str(&std::env::var("HOME").unwrap()).unwrap().join(PathBuf::from_str(".local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log")?);
    let log_path = arguments
        .game_log_file_path
        .as_ref()
        .unwrap_or(&default_log_path);
    let window_name = arguments.window_name.clone();
    let env = Env::default()
        .filter_or("WFINFO_LOG", "info")
        .write_style_or("WFINFO_STYLE", "always");
    Builder::from_env(env)
        .format_timestamp(None)
        .format_level(false)
        .format_module_path(false)
        .format_target(false)
        .init();

    let windows = Window::all()?;
    let Some(warframe_window) = windows.iter().find(|x| x.title() == window_name) else {
        return Err("Warframe window not found".into());
    };

    debug!(
        "Found window: {}x{}",
        warframe_window.width(),
        warframe_window.height()
    );

    let (prices, items) = fetch_prices_and_items()?;
    let db = Database::load_from_file(
        Some(&prices),
        Some(&items),
        Some(arguments.forma_platinum_multiplier),
        Some(arguments.forma_platinum_value),
    );

    info!("Loaded database");

    let (detection_sender, detection_receiver) = channel();
    let (snapit_sender, snapit_receiver) = channel();

    log_watcher(log_path.clone(), detection_sender.clone());

    // Setup hotkeys
    let _hotkey_manager = setup_hotkeys(
        arguments.detection_hotkey,
        arguments.snapit_hotkey,
        detection_sender,
        snapit_sender,
    )?;

    loop {
        // Check detection receiver
        match detection_receiver.try_recv() {
            Ok(()) => {
                run_detection(warframe_window, &db, &arguments);
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => break,
        }

        // Check snapit receiver
        match snapit_receiver.try_recv() {
            Ok(()) => {
                run_snapit(warframe_window, &db, &arguments);
            }
            Err(mpsc::TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(arguments.sleep_duration));
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }

    drop(OCR.lock().unwrap().take());
    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;
    use std::fs::read_to_string;

    use image::ImageReader;
    use indexmap::IndexMap;
    use rayon::prelude::*;
    use tesseract::Tesseract;
    use wfinfo::ocr::detect_theme;
    use wfinfo::ocr::extract_parts;
    use wfinfo::testing::Label;

    use super::*;

    #[test]
    fn single_image() {
        let image = ImageReader::open(format!("test-images/{}.png", 1))
            .unwrap()
            .decode()
            .unwrap();
        let text = reward_image_to_reward_names(image, None);
        let text = text.iter().map(|s| normalize_string(s));
        println!("{:#?}", text);
        let db = Database::load_from_file(None, None, Some(1.0), Some(35.0 / 3.0));
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
            let image = ImageReader::open("WFI test images/".to_string() + &filename)
                .unwrap()
                .decode()
                .unwrap();
            let text = reward_image_to_reward_names(image, None);
            let text: Vec<_> = text.iter().map(|s| normalize_string(s)).collect();
            println!("{:#?}", text);

            let db = Database::load_from_file(None, None, Some(1.0), Some(35.0 / 3.0));
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
                let image = ImageReader::open("WFI test images/".to_string() + &filename)
                    .unwrap()
                    .decode()
                    .unwrap();
                let text = reward_image_to_reward_names(image, None);
                let text: Vec<_> = text.iter().map(|s| normalize_string(s)).collect();
                println!("{:#?}", text);

                let db = Database::load_from_file(None, None, Some(1.0), Some(35.0 / 3.0));
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
            let image = ImageReader::open(format!("test-images/{}.png", i))
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
