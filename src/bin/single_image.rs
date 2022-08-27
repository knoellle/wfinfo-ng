use image::io::Reader;
use wfinfo::{
    database::Database,
    ocr::{image_to_strings, normalize_string},
};

fn main() {
    let image = Reader::open(std::env::args().nth(1).unwrap())
        .unwrap()
        .decode()
        .unwrap();
    let detections = image_to_strings(image.to_owned());
    println!("{:#?}", detections);
    let text = detections.iter().map(|s| normalize_string(s));
    println!("{:#?}", text);
    let db = Database::load_from_file(None);
    let items: Vec<_> = text.map(|s| db.find_item(&s, None)).collect();
    println!("{:#?}", items);
}
