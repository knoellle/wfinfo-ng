use captrs::Capturer;
use image::{io::Reader, DynamicImage, GrayImage, Rgb, RgbImage, Rgba};
use imageproc::{drawing::draw_hollow_rect_mut, rect::Rect};
use itertools::iproduct;
use wfinfo::{
    database::Database,
    ocr::{
        detect_theme, frame_to_image, image_to_string, normalize_string,
        reward_image_to_reward_names,
    },
    ocr_manual::image_to_string as image_to_string2,
    theme::Theme,
};

fn capture(capturer: &mut Capturer) -> image::DynamicImage {
    let frame = capturer.capture_frame().unwrap();
    println!("Captured");
    let dimensions = capturer.geometry();
    let image = DynamicImage::ImageRgb8(frame_to_image(dimensions, &frame));
    println!("Converted");

    image
}

fn threshold_with_theme(image: &DynamicImage, theme: &Theme) -> DynamicImage {
    let rgb = image.to_rgb8();
    DynamicImage::ImageRgb8(RgbImage::from_fn(
        image.width(),
        image.height(),
        |x, y| match theme.threshold_filter(*rgb.get_pixel(x, y)) {
            true => Rgb([0; 3]),
            false => Rgb([255; 3]),
        },
    ))
}

fn get_relic_name(relic_image: &DynamicImage, database: &Database) -> Option<String> {
    assert_eq!(relic_image.width(), 224);
    assert_eq!(relic_image.height(), 224);

    let height = 75;
    let name_area = relic_image.crop_imm(0, 224 - height, 224, height);
    name_area.save("relic.png").unwrap();

    Some(image_to_string(&name_area))
}

fn get_relic_count(relic_image: &DynamicImage, database: &Database) -> Option<String> {
    assert_eq!(relic_image.width(), 224);
    assert_eq!(relic_image.height(), 224);

    let name_area = relic_image.crop_imm(0, 8, 116, 40);
    name_area.save("relic.png").unwrap();

    Some(image_to_string(&name_area))
}

fn get_relic_positions() -> Vec<(u32, u32)> {
    iproduct!(0..4, 0..5)
        .map(|(y, x)| (130 + x * 288, 266 + y * 270))
        .collect()
}

fn load_templates() -> Vec<(char, GrayImage)> {
    ['4', 'l', 't', 'i'].into_iter()
        .map(|symbol| {
            (
                symbol,
                Reader::open(format!("templates/{symbol}.png"))
                    .unwrap()
                    .decode()
                    .unwrap()
                    .into_luma8(),
            )
        })
        .collect()
}

fn run_detection(capturer: &mut Capturer) {
    let mut image = Reader::open("relic.png").unwrap().decode().unwrap();
    let theme = detect_theme(&image);
    let relic_image = threshold_with_theme(&image, &theme).into_luma8();
    let templates = load_templates();
    let result = image_to_string2(&relic_image, &templates, 1.2);
    println!("{result:?}");

    return;
    let database = Database::load_from_file(None, None);

    // let image = capture(capturer);
    let mut image = Reader::open("relics-input.png").unwrap().decode().unwrap();
    let theme = detect_theme(&image);
    println!("Theme: {theme:?}");

    let mut count = 0;
    for (x, y) in get_relic_positions() {
        let relic_image = image.crop_imm(x, y, 224, 224);
        let relic_image = threshold_with_theme(&relic_image, &theme);
        relic_image.save(format!("relics/{x}-{y}.png")).unwrap();

        println!(
            "Name: {}",
            get_relic_name(&relic_image, &database).unwrap().trim()
        );
        println!(
            "Count: {}",
            get_relic_count(&relic_image, &database).unwrap().trim()
        );
        draw_hollow_rect_mut(
            &mut image,
            Rect::at(x as i32, y as i32).of_size(224, 224),
            Rgba::<u8>([255, 0, 0, 255]),
        );
        // if count == 1 {
        //     break
        // }
        // count += 1;
    }
    image.save("relics-output.png").unwrap();
}

fn main() {
    let mut capturer = Capturer::new(0).unwrap();
    run_detection(&mut capturer);
}
