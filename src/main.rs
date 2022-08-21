use std::f32::consts::PI;

use approx::AbsDiffEq;
use image::io::Reader;
use image::{DynamicImage, GenericImageView, Pixel, Rgb, Rgba};
use palette::{FromColor, Hsv, Srgb};
use tesseract::Tesseract;

const PIXEL_REWARD_WIDTH: usize = 968;
const PIXEL_REWARD_HEIGHT: usize = 235;
const PIXEL_REWARD_YDISPLAY: usize = 316;
const PIXEL_REWARD_LINE_HEIGHT: usize = 48;

enum Theme {
    Vitruvian,
    Stalker,
    Baruuk,
    Corpus,
    Fortuna,
    Grineer,
    Lotus,
    Nidus,
    Orokin,
    Tenno,
    HighContrast,
    Legacy,
    Equinox,
    DarkLotus,
    Zephyr,
    Unknown,
}

fn theme_threshold_filter(color: Rgb<u8>) -> bool {
    let rgb = Srgb::from_components((
        color.0[0] as f32 / 255.0,
        color.0[1] as f32 / 255.0,
        color.0[2] as f32 / 255.0,
    ));
    let hsv = Hsv::from_color(rgb);

    let primary = Hsv::from_color(Srgb::from_components((
        190.0 / 255.0,
        169.0 / 255.0,
        102.0 / 255.0,
    )));

    hsv.hue.abs_diff_eq(&primary.hue, 4.0) && hsv.saturation >= 0.25 && hsv.value >= 0.42
}

fn extract_parts(image: &DynamicImage) -> Vec<DynamicImage> {
    let screen_scaling = 1.0;
    let line_height = (PIXEL_REWARD_LINE_HEIGHT as f32 / 2.0 * screen_scaling) as usize;

    let width = 1920;
    let height = 1080;
    let most_width = (PIXEL_REWARD_WIDTH as f32 * screen_scaling) as usize;
    let most_left = (width / 2) - (most_width / 2);
    // Most Top = pixleRewardYDisplay - pixleRewardHeight + pixelRewardLineHeight
    //                   (316          -        235        +       44)    *    1.1    =    137
    let most_top = height / 2
        - ((PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT + PIXEL_REWARD_LINE_HEIGHT) as f32
            * screen_scaling) as usize;
    let most_bot = height / 2
        - ((PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT) as f32 * screen_scaling * 0.5) as usize;
    //Bitmap postFilter = new Bitmap(mostWidth, mostBot - mostTop);
    let rectangle = (most_left, most_top, most_width, most_bot - most_top);

    let mut prefilter = image.crop_imm(
        most_left as u32,
        most_top as u32,
        most_width as u32,
        (most_bot - most_top) as u32,
    );
    prefilter.save("test.png").unwrap();

    let mut rows = Vec::<usize>::new();
    for y in 0..prefilter.height() {
        let mut count = 0;
        for x in 0..prefilter.width() {
            let color = prefilter.get_pixel(x, y).to_rgb();
            if theme_threshold_filter(color) {
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

        println!("");
        println!("i: {}", i);
        println!("y_from_top: {}", y_from_top);
        let mut w = 0.0;
        for loc in text_top..text_top_bot + 1 {
            w += (scale_width as f32 * 0.06 - rows[y_from_top + loc] as f32).abs();
        }
        top_weights.push(w);

        let mut w = 0.0;
        for loc in text_top_bot + 1..text_both_bot {
            if rows[y_from_top + loc] < scale_width / 15 {
                w += (scale_width as f32 * 0.26 - rows[y_from_top + loc] as f32) * 5.0;
            } else {
                w += (scale_width as f32 * 0.24 - rows[y_from_top + loc] as f32).abs();
            }
        }
        mid_weights.push(w);

        let mut w = 0.0;
        for loc in text_both_bot..text_tail_bot {
            w += 10.0 * (scale_width as f32 * 0.007 - rows[y_from_top + loc] as f32).abs();
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

    println!("Scaling: {}", scaling);

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

    println!("top_five: {:?}", top_five);

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

    let mut prefilter = prefilter.into_rgb8();
    let partial_screenshot = DynamicImage::ImageRgb8(prefilter.clone()).crop_imm(
        crop_left as u32,
        crop_top as u32,
        crop_width as u32,
        crop_hei as u32,
    );

    // for (i, y) in top_five.iter().enumerate() {
    //     for x in 0..prefilter.width() {
    //         prefilter.put_pixel(x as u32, *y as u32, Rgb([i as u8 * 50, 0, 0]));
    //     }
    // }
    // prefilter.save("partial_screenshot.png").unwrap();

    partial_screenshot.save("partial_screenshot.png").unwrap();

    filter_and_separater_parts_from_part_box(partial_screenshot)
}

fn filter_and_separater_parts_from_part_box(image: DynamicImage) -> Vec<DynamicImage> {
    let mut filtered = image.into_rgb8();

    let mut weight = 0.0;
    let mut total_even = 0.0;
    let mut total_odd = 0.0;
    for x in 0..filtered.width() {
        let mut count = 0;
        for y in 0..filtered.height() {
            let pixel = filtered.get_pixel_mut(x, y);
            if theme_threshold_filter(*pixel) {
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

        println!("{}", cosine_thing);

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
    println!("Even: {}", total_even / total);
    println!("Odd: {}", total_odd / total);

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

fn main() {
    // let mut ocr = Tesseract::new(None, Some("eng")).expect("Could not initialize Tesseract");
    // ocr = ocr
    //     .set_image("test-images/1.png")
    //     .expect("Failed to set image");
    // let text = ocr.get_text().expect("Failed to get text");
    // println!("{}", text);
    let image = Reader::open("test-images/1.png").unwrap().decode().unwrap();
    let parts = extract_parts(&image);

    let mut ocr = Tesseract::new(None, Some("eng")).expect("Could not initialize Tesseract");
    for part in parts {
        let buffer = part.as_flat_samples_u8().unwrap();
        ocr = ocr
            // .set_image("test-images/1.png")
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
}
