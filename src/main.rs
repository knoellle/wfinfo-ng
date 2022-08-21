use approx::AbsDiffEq;
use image::io::Reader;
use image::{DynamicImage, GenericImageView, Rgba};
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

fn theme_threshold_filter(color: Rgba<u8>) -> bool {
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

fn extract_parts(image: &DynamicImage) {
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

    let prefilter = image.crop_imm(
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
            let color = prefilter.get_pixel(x, y);
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

    let partial_screenshot = prefilter.crop_imm(
        crop_left as u32,
        crop_top as u32,
        crop_width as u32,
        crop_hei as u32,
    );
    partial_screenshot.save("partial_screenshot.png");
}

fn main() {
    // let mut ocr = Tesseract::new(None, Some("eng")).expect("Could not initialize Tesseract");
    // ocr = ocr
    //     .set_image("test-images/1.png")
    //     .expect("Failed to set image");
    // let text = ocr.get_text().expect("Failed to get text");
    // println!("{}", text);
    let image = Reader::open("test-images/1.png").unwrap().decode().unwrap();
    extract_parts(&image);
}
