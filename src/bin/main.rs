use std::collections::HashMap;
use std::f32::consts::PI;
use std::fs::{self, read_to_string, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::sync::mpsc;
use std::thread::{sleep, sleep_ms};
use std::time::Duration;

use captrs::{Bgr8, Capturer};
use image::io::Reader;
use image::{DynamicImage, GenericImageView, ImageBuffer, Pixel, Rgb, RgbImage};
use notify::{watcher, RecursiveMode, Watcher};
use palette::{Pixel as PalettePixel, Srgb};
use tesseract::Tesseract;

use wfinfo::{database::Database, theme::Theme};

const PIXEL_REWARD_WIDTH: f32 = 968.0;
const PIXEL_REWARD_HEIGHT: f32 = 235.0;
const PIXEL_REWARD_YDISPLAY: f32 = 316.0;
const PIXEL_REWARD_LINE_HEIGHT: f32 = 48.0;

fn detect_theme(image: &DynamicImage) -> Theme {
    let screen_scaling = if image.width() * 9 > image.height() * 16 {
        image.height() as f32 / 1080.0
    } else {
        image.width() as f32 / 1920.0
    };

    let line_height = PIXEL_REWARD_LINE_HEIGHT / 2.0 * screen_scaling;
    let most_width = PIXEL_REWARD_WIDTH * screen_scaling;

    let min_width = most_width / 4.0;

    let mut weights: HashMap<Theme, f32> = HashMap::new();
    let mut debug_image = image.clone().into_rgb8();

    for y in line_height as u32..image.height() {
        let perc = (y as f32 - line_height) / (image.height() as f32 - line_height);
        let total_width = min_width * perc + min_width;
        for x in 0..total_width as u32 {
            let closest = Theme::closest_from_color(
                image
                    .get_pixel(x + (most_width - total_width) as u32 / 2, y)
                    .to_rgb(),
            );
            debug_image.put_pixel(x + (most_width - total_width) as u32 / 2, y, Rgb([255; 3]));

            *weights.entry(closest.0).or_insert(0.0) += 1.0 / (1.0 + closest.1).powi(4)
        }
    }

    debug_image.save("theme_detection.png").unwrap();

    println!("{:#?}", weights);

    *weights.iter().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0
}

fn extract_parts(image: &DynamicImage, theme: Theme) -> Vec<DynamicImage> {
    image.save("input.png").unwrap();
    let screen_scaling = if image.width() * 9 > image.height() * 16 {
        image.height() as f32 / 1080.0
    } else {
        image.width() as f32 / 1920.0
    };
    let line_height = (PIXEL_REWARD_LINE_HEIGHT as f32 / 2.0 * screen_scaling) as usize;

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
    //Bitmap postFilter = new Bitmap(mostWidth, mostBot - mostTop);
    let rectangle = (most_left, most_top, most_width, most_bot - most_top);

    let prefilter = image.crop_imm(
        most_left as u32,
        most_top as u32,
        most_width as u32,
        (most_bot - most_top) as u32,
    );
    let mut prefilter_draw = prefilter.clone().into_rgb8();
    prefilter.save("prefilter.png").unwrap();

    let mut rows = Vec::<usize>::new();
    for y in 0..prefilter.height() {
        let mut count = 0;
        for x in 0..prefilter.width() {
            let color = prefilter.get_pixel(x, y).to_rgb();
            if theme.threshold_filter(color) {
                count += 1;
            }
        }
        rows.push(count);
    }

    let mut perc_weights = Vec::new();
    let mut top_weights = Vec::new();
    let mut mid_weights = Vec::new();
    let mut bot_weights = Vec::new();

    let top_line_100 = prefilter.height() as usize - line_height;
    let top_line_50 = line_height / 2;

    let mut scaling = -1.0;
    let mut lowestWeight = 0.0;
    for i in 0..50 {
        let y_from_top = prefilter.height() as usize
            - (i as f32 * (top_line_100 - top_line_50) as f32 / 50.0 + top_line_50 as f32) as usize;
        let scale = 50 + i;
        let scale_width = (prefilter.width() as f32 * scale as f32 / 100.0) as usize;

        let text_segments = [2.0, 4.0, 16.0, 21.0];
        let text_top = (screen_scaling * text_segments[0] * scale as f32 / 100.0) as usize;
        let text_top_bot = (screen_scaling * text_segments[1] * scale as f32 / 100.0) as usize;
        let text_both_bot = (screen_scaling * text_segments[2] * scale as f32 / 100.0) as usize;
        let text_tail_bot = (screen_scaling * text_segments[3] * scale as f32 / 100.0) as usize;

        // println!("");
        // println!("i: {}", i);
        // println!("y_from_top: {}", y_from_top);
        let mut w = 0.0;
        for loc in text_top..text_top_bot + 1 {
            w += (scale_width as f32 * 0.06 - rows[y_from_top + loc] as f32).abs();
            prefilter_draw.put_pixel(
                prefilter_draw.width() / 2 + i as u32,
                (y_from_top + loc) as u32,
                Rgb([255; 3]),
            );
        }
        top_weights.push(w);

        let mut w = 0.0;
        for loc in text_top_bot + 1..text_both_bot {
            if rows[y_from_top + loc] < scale_width / 15 {
                w += (scale_width as f32 * 0.26 - rows[y_from_top + loc] as f32) * 5.0;
            } else {
                w += (scale_width as f32 * 0.24 - rows[y_from_top + loc] as f32).abs();
            }
            prefilter_draw.put_pixel(
                prefilter_draw.width() / 2 + i as u32,
                (y_from_top + loc) as u32,
                Rgb([0, 255, 0]),
            );
        }
        mid_weights.push(w);

        let mut w = 0.0;
        for loc in text_both_bot..text_tail_bot {
            w += 10.0 * (scale_width as f32 * 0.007 - rows[y_from_top + loc] as f32).abs();
            prefilter_draw.put_pixel(
                prefilter_draw.width() / 2 + i as u32,
                (y_from_top + loc) as u32,
                Rgb([0, 0, 255]),
            );
        }
        bot_weights.push(w);

        top_weights[i] /= (text_top_bot - text_top + 1) as f32;
        mid_weights[i] /= (text_both_bot - text_top_bot - 2) as f32;
        bot_weights[i] /= (text_tail_bot - text_both_bot - 1) as f32;
        perc_weights.push(top_weights[i] + mid_weights[i] + bot_weights[i]);

        if scaling <= 0.0 || lowestWeight > perc_weights[i] {
            scaling = scale as f32;
            lowestWeight = perc_weights[i];
        }
    }

    // println!("Scaling: {}", scaling);

    let mut top_five = [-1_isize; 5];
    for (i, _w) in perc_weights.iter().enumerate() {
        let mut slot: isize = 4;
        while slot != -1
            && top_five[slot as usize] != -1
            && perc_weights[i] > perc_weights[top_five[slot as usize] as usize]
        {
            slot -= 1;
        }

        if slot != -1 {
            for slot2 in 0..slot {
                top_five[slot2 as usize] = top_five[slot2 as usize + 1]
            }
            top_five[slot as usize] = i as isize;
        }
    }

    // println!("top_five: {:?}", top_five);

    scaling = scaling / 100.0;
    let high_scaling = if scaling < 1.0 {
        scaling + 0.01
    } else {
        scaling
    };
    let low_scaling = if scaling > 0.5 {
        scaling + 0.01
    } else {
        scaling
    };

    let crop_width = PIXEL_REWARD_WIDTH as f32 * screen_scaling * high_scaling;
    let crop_left = prefilter.width() as f32 / 2.0 - crop_width / 2.0;
    let crop_top = height as f32 / 2.0
        - (PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT + PIXEL_REWARD_LINE_HEIGHT) as f32
            * screen_scaling
            * high_scaling;
    let crop_bot = height as f32 / 2.0
        - (PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT) as f32 * screen_scaling * low_scaling;
    let crop_hei = crop_bot - crop_top;
    let crop_top = crop_top - most_top as f32;

    let partial_screenshot = DynamicImage::ImageRgb8(prefilter.into_rgb8()).crop_imm(
        crop_left as u32,
        crop_top as u32,
        crop_width as u32,
        crop_hei as u32,
    );

    // Draw top 5
    for (i, y) in top_five.iter().enumerate() {
        for x in 0..prefilter_draw.width() {
            prefilter_draw.put_pixel(x as u32, *y as u32, Rgb([255 - i as u8 * 50, 0, 0]));
        }
    }
    // Draw histogram
    for (y, row) in rows.iter().enumerate() {
        for x in 0..*row {
            prefilter_draw.put_pixel(x as u32, y as u32, Rgb([0, 255, 0]));
        }
    }

    prefilter_draw.save("prefilter.png").unwrap();

    partial_screenshot.save("partial_screenshot.png").unwrap();

    filter_and_separater_parts_from_part_box(partial_screenshot, theme)
}

fn filter_and_separater_parts_from_part_box(
    image: DynamicImage,
    theme: Theme,
) -> Vec<DynamicImage> {
    let mut filtered = image.into_rgb8();

    let mut weight = 0.0;
    let mut total_even = 0.0;
    let mut total_odd = 0.0;
    for x in 0..filtered.width() {
        let mut count = 0;
        for y in 0..filtered.height() {
            let pixel = filtered.get_pixel_mut(x, y);
            if theme.threshold_filter(*pixel) {
                *pixel = Rgb([0; 3]);
                count += 1;
            } else {
                *pixel = Rgb([255; 3]);
            }
        }

        count = count.min(filtered.height() / 3);
        let cosine = (8.0 * x as f32 * PI / filtered.width() as f32).cos();
        let cosine_thing = cosine.powi(3);

        // filtered.put_pixel(
        //     x,
        //     ((cosine_thing / 2.0 + 0.5) * (filtered.height() - 1) as f32) as u32,
        //     Rgb([255, 0, 0]),
        // );

        // println!("{}", cosine_thing);

        let this_weight = cosine * count as f32;
        weight += this_weight;

        if cosine < 0.0 {
            total_even -= this_weight;
        } else if cosine > 0.0 {
            total_odd += this_weight;
        }
    }

    filtered
        .save("filtered.png")
        .expect("Failed to write filtered image");

    if total_even == 0.0 && total_odd == 0.0 {
        return vec![];
    }

    let total = total_even + total_odd;
    // println!("Even: {}", total_even / total);
    // println!("Odd: {}", total_odd / total);

    let box_width = filtered.width() / 4;
    let box_height = filtered.height();

    let mut curr_left = 0;
    let mut player_count = 4;

    if total_odd > total_even {
        curr_left = box_width / 2;
        player_count = 3;
    }

    let mut images = Vec::new();

    let dynamic_image = DynamicImage::ImageRgb8(filtered);
    for i in 0..player_count {
        let cropped = dynamic_image.crop_imm(curr_left + i * box_width, 0, box_width, box_height);
        cropped
            .save(format!("part-{}.png", i))
            .expect("Failed to write image");
        images.push(cropped);
    }

    images
}

fn frame_to_image(dimensions: (u32, u32), frame: &[Bgr8]) -> RgbImage {
    let container = frame
        .iter()
        .flat_map(|bgr8| [bgr8.r, bgr8.g, bgr8.b])
        .collect();
    RgbImage::from_raw(dimensions.0, dimensions.1, container).unwrap()
}

fn image_to_strings(image: DynamicImage) -> Vec<String> {
    let theme = detect_theme(&image);
    println!("Theme: {:?}", theme);
    let parts = extract_parts(&image, theme);
    println!("Extracted part images");

    parts
        .iter()
        .map(|part| {
            let mut ocr =
                Tesseract::new(None, Some("eng")).expect("Could not initialize Tesseract");
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
            text
        })
        .collect()
}

fn normalize_string(string: &str) -> String {
    string.replace(|c: char| !c.is_ascii_alphabetic(), "")
}

#[cfg(test)]
mod test {
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
            "WFI test images/FullScreenShot 2020-02-22 14-48-5430.png",
            "WFI test images/FullScreenShot 2020-06-18 19-10-1443.png",
            "WFI test images/FullScreenShot 2020-06-20 19-34-4299.png",
            "WFI test images/FullScreenShot 2020-06-20 19-38-2502.png",
            "WFI test images/FullScreenShot 2020-06-20 20-09-5411.png",
            "WFI test images/FullScreenShot 2020-06-20 20-14-0448.png",
            "WFI test images/FullScreenShot 2020-06-20 20-18-4525.png",
            "WFI test images/FullScreenShot 2020-06-20 20-20-0744.png",
            "WFI test images/FullScreenShot 2020-06-20 22-56-4320.png",
            "WFI test images/FullScreenShot 2020-06-21 20-09-3214.png",
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
            "WFI test images/FullScreenShot 2020-06-30 11-48-1379.png",
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
            "WFI test images/FullScreenShot 2020-06-30 13-39-5708.png",
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
            "WFI test images/FullScreenShot 2020-06-30 17-20-0497.png",
            "WFI test images/FullScreenShot 2020-06-30 17-24-2319.png",
            "WFI test images/FullScreenShot 2020-06-30 17-29-0636.png",
            "WFI test images/FullScreenShot 2020-06-30 17-33-2737.png",
            "WFI test images/FullScreenShot 2020-06-30 17-37-4678.png",
            "WFI test images/FullScreenShot 2020-06-30 17-42-2817.png",
            "WFI test images/FullScreenShot 2020-06-30 17-47-3722.png",
            "WFI test images/FullScreenShot 2020-06-30 17-53-0962.png",
            "WFI test images/FullScreenShot 2020-06-30 17-56-0832.png",
            "WFI test images/FullScreenShot 2020-06-30 18-00-0982.png",
            "WFI test images/FullScreenShot 2020-06-30 18-09-1947.png",
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
        for filename in filenames {
            let image = Reader::open(filename).unwrap().decode().unwrap();
            let text = image_to_strings(image);
            let text: Vec<_> = text.iter().map(|s| normalize_string(s)).collect();
            println!("{:#?}", text); // TODO: This prints wrong strings!!!
            let db = Database::load_from_file(None);
            let items: Vec<_> = text.iter().map(|s| db.find_item(&s, None)).collect();
            println!("{:#?}", items);
            println!("{}", filename);
            assert!(items.iter().all(|item| item.is_some()));
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
            println!("{}\n\t{}", item.name, item.custom_avg);
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
