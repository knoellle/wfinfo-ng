use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::sync::mpsc;
use std::thread::sleep;
use std::time::Duration;

use captrs::Capturer;
use image::DynamicImage;
use notify::{watcher, RecursiveMode, Watcher};
use wfinfo::database::Database;
use wfinfo::ocr::{frame_to_image, image_to_strings, normalize_string};

#[cfg(test)]
mod test {
    use std::fs::read_to_string;

    use image::io::Reader;
    use indexmap::IndexMap;
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
        let text = image_to_strings(image);
        let text = text.iter().map(|s| normalize_string(s));
        println!("{:#?}", text);
        let db = Database::load_from_file(None);
        let items: Vec<_> = text.map(|s| db.find_item(&s, None)).collect();
        println!("{:#?}", items);

        assert_eq!(
            items[0].expect("Didn't find an item?").name,
            "Octavia Prime Systems Blueprint"
        );
        assert_eq!(
            items[1].expect("Didn't find an item?").name,
            "Octavia Prime Blueprint"
        );
        assert_eq!(
            items[2].expect("Didn't find an item?").name,
            "Tenora Prime Blueprint"
        );
        assert_eq!(
            items[3].expect("Didn't find an item?").name,
            "Harrow Prime Systems Blueprint"
        );
    }

    #[test]
    fn wfi_images() {
        let filenames = [
            // "WFI test images/FullScreenShot 2020-02-22 14-48-5430.png", // Scaling issue
            // "WFI test images/FullScreenShot 2020-06-18 19-10-1443.png", // Kuva stuff
            "WFI test images/FullScreenShot 2020-06-20 19-34-4299.png",
            "WFI test images/FullScreenShot 2020-06-20 19-38-2502.png",
            "WFI test images/FullScreenShot 2020-06-20 20-09-5411.png",
            "WFI test images/FullScreenShot 2020-06-20 20-14-0448.png",
            "WFI test images/FullScreenShot 2020-06-20 20-18-4525.png",
            "WFI test images/FullScreenShot 2020-06-20 20-20-0744.png",
            "WFI test images/FullScreenShot 2020-06-20 22-56-4320.png",
            // "WFI test images/FullScreenShot 2020-06-21 20-09-3214.png", // high contrast
            "WFI test images/FullScreenShot 2020-06-22 16-45-2295.png",
            "WFI test images/FullScreenShot 2020-06-26 20-48-3752.png",
            "WFI test images/FullScreenShot 2020-06-27 15-10-2630.png",
            "WFI test images/FullScreenShot 2020-06-30 10-58-4234.png",
            "WFI test images/FullScreenShot 2020-06-30 11-09-1971.png",
            "WFI test images/FullScreenShot 2020-06-30 11-12-2629.png",
            "WFI test images/FullScreenShot 2020-06-30 11-15-5274.png",
            "WFI test images/FullScreenShot 2020-06-30 11-19-5866.png",
            "WFI test images/FullScreenShot 2020-06-30 11-24-2100.png",
            "WFI test images/FullScreenShot 2020-06-30 11-27-2797.png",
            "WFI test images/FullScreenShot 2020-06-30 11-30-5155.png",
            "WFI test images/FullScreenShot 2020-06-30 11-37-4636.png",
            "WFI test images/FullScreenShot 2020-06-30 11-40-5599.png",
            "WFI test images/FullScreenShot 2020-06-30 11-45-0070.png",
            "WFI test images/FullScreenShot 2020-06-30 11-48-1379.png", // Atlas detected as Ash
            "WFI test images/FullScreenShot 2020-06-30 11-52-2415.png",
            "WFI test images/FullScreenShot 2020-06-30 11-57-1724.png",
            "WFI test images/FullScreenShot 2020-06-30 12-38-5685.png",
            "WFI test images/FullScreenShot 2020-06-30 12-41-3594.png",
            "WFI test images/FullScreenShot 2020-06-30 12-45-1337.png",
            "WFI test images/FullScreenShot 2020-06-30 12-49-2454.png",
            "WFI test images/FullScreenShot 2020-06-30 12-54-0179.png",
            "WFI test images/FullScreenShot 2020-06-30 12-57-1837.png",
            "WFI test images/FullScreenShot 2020-06-30 13-00-5126.png",
            "WFI test images/FullScreenShot 2020-06-30 13-03-5934.png",
            "WFI test images/FullScreenShot 2020-06-30 13-32-2693.png",
            "WFI test images/FullScreenShot 2020-06-30 13-35-3571.png",
            "WFI test images/FullScreenShot 2020-06-30 13-39-5708.png", // wrong theme detected
            "WFI test images/FullScreenShot 2020-06-30 13-43-4962.png",
            "WFI test images/FullScreenShot 2020-06-30 13-47-3641.png",
            "WFI test images/FullScreenShot 2020-06-30 14-39-5467.png",
            "WFI test images/FullScreenShot 2020-06-30 14-43-3028.png",
            "WFI test images/FullScreenShot 2020-06-30 14-48-4323.png",
            "WFI test images/FullScreenShot 2020-06-30 14-59-2275.png",
            "WFI test images/FullScreenShot 2020-06-30 15-02-3402.png",
            "WFI test images/FullScreenShot 2020-06-30 15-12-2945.png",
            "WFI test images/FullScreenShot 2020-06-30 15-16-4411.png",
            "WFI test images/FullScreenShot 2020-06-30 15-24-0499.png",
            "WFI test images/FullScreenShot 2020-06-30 15-30-4981.png",
            "WFI test images/FullScreenShot 2020-06-30 17-20-0497.png", // Nyx detected as Bo
            "WFI test images/FullScreenShot 2020-06-30 17-24-2319.png",
            "WFI test images/FullScreenShot 2020-06-30 17-29-0636.png",
            "WFI test images/FullScreenShot 2020-06-30 17-33-2737.png",
            "WFI test images/FullScreenShot 2020-06-30 17-37-4678.png",
            "WFI test images/FullScreenShot 2020-06-30 17-42-2817.png",
            "WFI test images/FullScreenShot 2020-06-30 17-47-3722.png",
            "WFI test images/FullScreenShot 2020-06-30 17-53-0962.png",
            "WFI test images/FullScreenShot 2020-06-30 17-56-0832.png",
            "WFI test images/FullScreenShot 2020-06-30 18-00-0982.png",
            "WFI test images/FullScreenShot 2020-06-30 18-09-1947.png", // Helios systems detected as set
            "WFI test images/FullScreenShot 2020-06-30 18-12-1813.png",
            "WFI test images/FullScreenShot 2020-06-30 18-15-2892.png",
            "WFI test images/FullScreenShot 2020-06-30 18-18-3724.png",
            "WFI test images/FullScreenShot 2020-06-30 18-21-5952.png",
            "WFI test images/FullScreenShot 2020-06-30 18-25-0517.png",
            "WFI test images/FullScreenShot 2020-06-30 18-28-4182.png",
            "WFI test images/FullScreenShot 2020-06-30 18-31-5444.png",
            "WFI test images/FullScreenShot 2020-06-30 18-35-2729.png",
            "WFI test images/FullScreenShot 2020-06-30 18-40-3237.png",
            "WFI test images/FullScreenShot 2020-06-30 18-43-5774.png",
            "WFI test images/FullScreenShot 2020-06-30 18-47-3461.png",
            "WFI test images/FullScreenShot 2020-06-30 19-01-2231.png",
            "WFI test images/FullScreenShot 2020-06-30 19-04-4056.png",
            "WFI test images/FullScreenShot 2020-06-30 19-08-1329.png",
            "WFI test images/FullScreenShot 2020-06-30 19-16-0396.png",
            "WFI test images/FullScreenShot 2020-06-30 19-24-5871.png",
            "WFI test images/FullScreenShot 2020-06-30 19-29-0564.png",
            "WFI test images/FullScreenShot 2020-06-30 19-32-3442.png",
            "WFI test images/FullScreenShot 2020-06-30 19-36-0217.png",
            "WFI test images/FullScreenShot 2020-06-30 19-49-2217.png",
            "WFI test images/FullScreenShot 2020-06-30 19-52-2884.png",
            "WFI test images/FullScreenShot 2020-06-30 19-55-2891.png",
            "WFI test images/FullScreenShot 2020-06-30 20-02-5516.png",
            "WFI test images/FullScreenShot 2020-06-30 20-06-2083.png",
            "WFI test images/FullScreenShot_2020-02-05_13-25-4618.png",
        ];
        let labels: IndexMap<String, Label> =
            serde_json::from_str(&read_to_string("WFI test images/labels.json").unwrap()).unwrap();
        for (filename, label) in labels {
            let image = Reader::open("WFI test images/".to_string() + &filename)
                .unwrap()
                .decode()
                .unwrap();
            let text = image_to_strings(image);
            let text: Vec<_> = text.iter().map(|s| normalize_string(s)).collect();
            println!("{:#?}", text);

            let db = Database::load_from_file(None);
            let items: Vec<_> = text.iter().map(|s| db.find_item(&s, None)).collect();
            println!("{:#?}", items);
            println!("{}", filename);

            let item_names = items.iter().map(|item| item.map(|item| item.name.clone()));

            for (result, expectation) in item_names.zip(label.items) {
                if expectation.is_empty() {
                    assert_eq!(result, None)
                } else {
                    assert_eq!(result, Some(expectation))
                }
            }
        }
    }

    // #[test]
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

fn run_detection(capturer: &mut Capturer) {
    let frame = capturer.capture_frame().unwrap();
    println!("Captured");
    let dimensions = capturer.geometry();
    let image = DynamicImage::ImageRgb8(frame_to_image(dimensions, &frame));
    println!("Converted");
    let text = image_to_strings(image.clone());
    let text = text.iter().map(|s| normalize_string(s));
    println!("{:#?}", text);
    let db = Database::load_from_file(None);
    let items: Vec<_> = text.map(|s| db.find_item(&s, None)).collect();
    for item in items {
        if let Some(item) = item {
            println!("{}\n\t{}", item.name, item.platinum);
        } else {
            println!("Unknown item\n\tUnknown");
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    println!("Path: {}", path);
    let (tx, rx) = mpsc::channel();
    let mut watcher = watcher(tx, Duration::from_millis(100)).unwrap();
    watcher.watch(&path, RecursiveMode::NonRecursive).unwrap();

    let mut position = File::open(&path).unwrap().seek(SeekFrom::End(0)).unwrap();
    println!("Position: {}", position);

    let mut capturer = Capturer::new(0).unwrap();
    println!("Capture source resolution: {:?}", capturer.geometry());

    loop {
        match rx.recv() {
            Ok(notify::DebouncedEvent::Write(_)) => {
                let mut f = File::open(&path).unwrap();
                f.seek(SeekFrom::Start(position)).unwrap();

                let mut reward_screen_detected = false;

                let reader = BufReader::new(f.by_ref());
                for line in reader.lines() {
                    let line = line.unwrap();
                    // println!("> {:?}", line);
                    if line.contains("Pause countdown done")
                        || line.contains("Got rewards")
                        || line.contains("Created /Lotus/Interface/ProjectionRewardChoice.swf")
                    {
                        reward_screen_detected = true;
                    }
                }

                if reward_screen_detected {
                    sleep(Duration::from_millis(500));
                    run_detection(&mut capturer);
                }

                position = f.metadata().unwrap().len();
                println!("{}", position);
                // position = reader.stream_position().unwrap();
            }
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
}
