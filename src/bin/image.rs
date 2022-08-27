use std::{fs::write, path::PathBuf};

use image::io::Reader;
use indexmap::IndexMap;
use serde::Serialize;
use wfinfo::{
    database::Database,
    ocr::{detect_theme, image_to_strings, normalize_string},
    testing::Label,
    theme::Theme,
};

fn main() {
    let mut labels = IndexMap::new();

    for argument in std::env::args().skip(1) {
        let filepath = PathBuf::from(argument);
        let image = Reader::open(&filepath).unwrap().decode().unwrap();

        let detections = image_to_strings(image.clone());
        println!("{:#?}", detections);

        let text = detections.iter().map(|s| normalize_string(s));
        println!("{:#?}", text);

        let db = Database::load_from_file(None);
        let items: Vec<_> = text.map(|s| db.find_item(&s, None)).collect();
        for item in items.iter() {
            if let Some(item) = item {
                println!("{}\n", item.name);
            } else {
                println!("Unknown item\n");
            }
        }
        let item_names = items
            .iter()
            .map(|item| {
                item.map(|item| item.name.clone())
                    .unwrap_or_else(|| "ERROR".to_string())
            })
            .collect();
        let theme = detect_theme(&image);
        labels.insert(
            filepath
                .file_name()
                .unwrap()
                .to_owned()
                .to_string_lossy()
                .to_string(),
            Label {
                theme,
                items: item_names,
            },
        );
        println!("{:?}", filepath);

        // let mut buffer = "".to_string();
        // stdin().read_line(&mut buffer).unwrap();
    }

    let labels_json = serde_json::to_string_pretty(&labels).unwrap();
    write("labels.json", labels_json).unwrap();
}
