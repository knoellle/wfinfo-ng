use std::error::Error;
use std::thread::sleep;
use std::time::Duration;
use std::{fs::File, thread};
use std::{
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    sync::mpsc::channel,
};

use std::{path::PathBuf, sync::mpsc};
use clap::{Parser, ArgAction};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use image::DynamicImage;
use notify::{watcher, RecursiveMode, Watcher};
use serde_json::json;
use xcap::Window;
use dirs::home_dir;

use wfinfo::{
    database::Database,
    ocr::{normalize_string, reward_image_to_reward_names, OCR},
};

fn run_detection(capturer: &Window, db: &Database, json_output:bool) {
    let frame = capturer.capture_image().unwrap();
    println!("Captured");
    let image = DynamicImage::ImageRgba8(frame);
    println!("Converted");
    let text = reward_image_to_reward_names(image, None);
    let text = text.iter().map(|s| normalize_string(s));
    println!("{:#?}", text);

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
    
    if json_output {
        println!("begin json");
        for (index, item) in items.iter().enumerate() {
        
            if let Some(item) = item {
                println!(
                    "{{\"name\": \"{}\", \"platinum\": {}, \"ducats\": {}, \"best\": {}}}",
                    item.drop_name,
                    item.platinum,
                    item.ducats as f32 / 10.0,
                    if Some(index) == best { "true" } else { "false" }
                );
            } else {
                println!("{{\"name\": \"Unknown item\"}}");
            }
        }
        println!("end json");
        return;
    }
    else {
        for (index, item) in items.iter().enumerate() {
            if let Some(item) = item {
                println!(
                    "{}\n\t{}\t{}\t{}",
                    item.drop_name,
                    item.platinum,
                    item.ducats as f32 / 10.0,
                    if Some(index) == best { "<----" } else { "" }
                );
            } else {
                println!("Unknown item\n\tUnknown");
            }
        }
    }
}

fn log_watcher(path: PathBuf, event_sender: mpsc::Sender<()>) {
    println!("Path: {}", path.display());

    thread::spawn(move || {
        let mut position = File::open(&path).unwrap().seek(SeekFrom::End(0)).unwrap();
        println!("Position: {}", position);

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
                                println!("Error reading line: {}", err);
                                continue;
                            }
                        };
                        // println!("> {:?}", line);
                        if line.contains("Pause countdown done")
                            || line.contains("Got rewards")
                            || line.contains("Created /Lotus/Interface/ProjectionRewardChoice.swf")
                        {
                            reward_screen_detected = true;
                        }
                    }

                    if reward_screen_detected {
                        println!("Detected, waiting...");
                        sleep(Duration::from_millis(1500));
                        event_sender.send(()).unwrap();
                    }

                    position = f.metadata().unwrap().len();
                    println!("Log position: {}", position);
                }
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                }
            }
        }
    });
}

fn hotkey_watcher(hotkey: HotKey, event_sender: mpsc::Sender<()>) {
    println!("watching hotkey: {hotkey:?}");
    thread::spawn(move || {
        let manager = GlobalHotKeyManager::new().unwrap();
        manager.register(hotkey).unwrap();

        while let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
            println!("{:?}", event);
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

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = "Warframe reward detection tool for linux")]
struct Args {
    /// enable JSON output
    #[arg(short, long)]
    json: bool,
    /// define ee.log path if not default
    #[arg(short, long)]
    path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    
    let home_dir = home_dir().expect("Could not find home directory");
    let default_ee_log_path = home_dir.join(".steam/steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log");

    let windows = Window::all()?;
    let db = Database::load_from_file(None, None);
    let Some(warframe_window) = windows.iter().find(|x| x.title() == "Warframe") else {
        return Err("Warframe window not found".into());
    };

    println!(
        "Capture source resolution: {:?}x{:?}",
        warframe_window.width(),
        warframe_window.height()
    );

    println!("Loaded database");

    let (event_sender, event_receiver) = channel();
    
    //TODO this can be handled better
    if let Some(ee_log_path) = args.path {
        println!("Using custom ee.log path: {}", ee_log_path.display());
        log_watcher(ee_log_path.into(), event_sender.clone());
    } else {
        println!("Using default ee.log path: {}", default_ee_log_path.display());
        log_watcher(default_ee_log_path.into(), event_sender.clone());
    }
    
    hotkey_watcher("F12".parse()?, event_sender);

    while let Ok(()) = event_receiver.recv() {
        println!("Capturing");
        run_detection(warframe_window, &db, args.json);
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
        println!("text");
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
