use lazy_static::lazy_static;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::f32::consts::PI;
use std::{collections::HashMap, sync::Mutex};
use tesseract::Tesseract;

use image::imageops::colorops;
use image::{DynamicImage, GenericImageView, Pixel, Rgb};
use log::debug;

use crate::theme::Theme;

// Constants for reward screen dimensions in pixels at 1080p resolution
const PIXEL_REWARD_WIDTH: f32 = 968.0;
const PIXEL_REWARD_HEIGHT: f32 = 235.0;
const PIXEL_REWARD_YDISPLAY: f32 = 316.0;
const PIXEL_REWARD_LINE_HEIGHT: f32 = 48.0;

/// Represents a selected region on the screen
pub struct Selection {
    pub x: i32,      // Absolute X coordinate
    pub y: i32,      // Absolute Y coordinate
    pub width: i32,  // Width of selection
    pub height: i32, // Height of selection
}

/// Converts raw slop output (WxH+X+Y format) into a Selection struct
/// Example input: "100x200+300+400" -> Selection of 100x200 at (300,400)
pub fn slop_to_selection(slop_output: &str) -> Option<Selection> {
    // Parse the WxH+X+Y format, trimming any whitespace/newlines
    let (dimensions, coordinates) = slop_output.trim().split_once('+')?;
    let (width, height) = dimensions.split_once('x')?;
    let (x, y) = coordinates.split_once('+')?;

    let x = x.parse().ok()?;
    let y = y.parse().ok()?;
    let width = width.parse().ok()?;
    let height = height.parse().ok()?;

    debug!("Selection: {}x{} at {},{}", width, height, x, y);
    Some(Selection {
        x,
        y,
        width,
        height,
    })
}

/// Extracts a part of an image and enhances it for OCR
///
/// # Arguments
/// * `image` - Source image to extract from
/// * `sel_size` - (width, height) of the selection
/// * `sel_pos` - (x, y) coordinates relative to the image
/// * `brightness` - Brightness adjustment (-255 to 255)
/// * `contrast` - Contrast adjustment (0.0 to 10.0)
pub fn extract_part(
    image: &DynamicImage,
    sel_size: (i32, i32),
    sel_pos: (i32, i32),
    brightness: i32,
    contrast: f32,
) -> DynamicImage {
    debug!("Processing image {}x{}", image.width(), image.height());

    // Convert coordinates and ensure they're within bounds
    let x = sel_pos.0.max(0) as u32;
    let y = sel_pos.1.max(0) as u32;
    let width = sel_size.0.max(0) as u32;
    let height = sel_size.1.max(0) as u32;

    let width = width.min(image.width() - x);
    let height = height.min(image.height() - y);

    debug!(
        "Cropping region: x={}, y={}, w={}, h={}",
        x, y, width, height
    );
    let cropped = image.crop_imm(x, y, width, height);

    // Two-step image enhancement for better OCR:
    // 1. Brighten to make text more visible
    // 2. Increase contrast to separate text from background
    let rgb = cropped.into_rgb8();
    let brightened = colorops::brighten(&rgb, brightness);
    let enhanced = colorops::contrast(&brightened, contrast);
    let enhanced = DynamicImage::ImageRgb8(enhanced);

    enhanced
}

/// Performs OCR on an image using Tesseract
///
/// # Arguments
/// * `tesseract` - Mutable reference to the Tesseract instance
/// * `image` - Image to perform OCR on
pub fn image_to_string(tesseract: &mut Option<Tesseract>, image: &DynamicImage) -> String {
    debug!("Running OCR on {}x{} image", image.width(), image.height());
    let mut ocr = tesseract.take().unwrap();

    // Convert image to format required by Tesseract
    let buffer = image.as_flat_samples_u8().unwrap();
    ocr = ocr
        .set_frame(
            buffer.samples,
            image.width() as i32,
            image.height() as i32,
            3,                        // RGB format (3 channels)
            3 * image.width() as i32, // Bytes per row (RGB * width)
        )
        .expect("Failed to set image");

    let result = ocr.get_text().expect("Failed to get text");
    debug!("OCR result: {}", result.trim());
    tesseract.replace(ocr);

    result
}

/// Parameters for OCR selection processing
pub struct SelectionParams {
    pub abs_x: i32,
    pub abs_y: i32,
    pub width: i32,
    pub height: i32,
    pub monitor_x: i32,
    pub monitor_y: i32,
    pub brightness: i32,
    pub contrast: f32,
}

/// Converts absolute screen coordinates to image-relative coordinates and performs OCR
///
/// # Arguments
/// * `image` - Source image
/// * `params` - Selection parameters including coordinates, dimensions, and OCR settings
pub fn selection_to_part_name(image: DynamicImage, params: SelectionParams) -> Option<String> {
    // Convert absolute screen coordinates to image-relative coordinates
    let x = params.abs_x - params.monitor_x;
    let y = params.abs_y - params.monitor_y;
    debug!(
        "Processing selection: {}x{} at {},{}",
        params.width, params.height, x, y
    );

    let text = part_image_to_part_name(
        image,
        None,
        (params.width, params.height),
        (x, y),
        params.brightness,
        params.contrast,
    );
    if text.trim().is_empty() {
        debug!("No text detected in selection");
        return None;
    }

    Some(text)
}

/// Removes all non-ASCII-alphabetic characters from a string
/// Used to normalize item names for database lookup
pub fn normalize_string(string: &str) -> String {
    string.replace(|c: char| !c.is_ascii_alphabetic(), "")
}

/// Detects the theme of a reward screen by analyzing pixel colors
///
/// This function works by:
/// 1. Calculating screen scaling based on resolution
/// 2. Sampling pixels in the expected reward area
/// 3. Comparing colors to known theme colors
/// 4. Returning the most prevalent theme
pub fn detect_theme(image: &DynamicImage) -> Theme {
    let screen_scaling = if image.width() * 9 > image.height() * 16 {
        image.height() as f32 / 1080.0
    } else {
        image.width() as f32 / 1920.0
    };

    let line_height = PIXEL_REWARD_LINE_HEIGHT / 2.0 * screen_scaling;
    let most_width = PIXEL_REWARD_WIDTH * screen_scaling;

    let min_width = most_width / 4.0;

    let weights = (line_height as u32..image.height())
        .into_par_iter()
        .fold(HashMap::new, |mut weights: HashMap<Theme, f32>, y| {
            let perc = (y as f32 - line_height) / (image.height() as f32 - line_height);
            let total_width = min_width * perc + min_width;
            for x in 0..total_width as u32 {
                let closest = Theme::closest_from_color(
                    image
                        .get_pixel(x + (most_width - total_width) as u32 / 2, y)
                        .to_rgb(),
                );

                *weights.entry(closest.0).or_insert(0.0) += 1.0 / (1.0 + closest.1).powi(4)
            }
            weights
        })
        .reduce(HashMap::new, |mut a, b| {
            for (k, v) in b {
                *a.entry(k).or_insert(0.0) += v;
            }
            a
        });

    debug!("{:#?}", weights);

    weights
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap()
        .0
        .to_owned()
}

/// Extracts individual reward parts from a reward screen image
///
/// This function:
/// 1. Calculates screen scaling to handle different resolutions
/// 2. Crops the image to the reward area
/// 3. Analyzes pixel rows to find text regions
/// 4. Uses scaling detection to handle UI size variations
/// 5. Returns a vector of cropped images, one for each reward
pub fn extract_parts(image: &DynamicImage, theme: Theme) -> Vec<DynamicImage> {
    image.save("input.png").unwrap();
    let screen_scaling = if image.width() * 9 > image.height() * 16 {
        image.height() as f32 / 1080.0
    } else {
        image.width() as f32 / 1920.0
    };
    let line_height = (PIXEL_REWARD_LINE_HEIGHT / 2.0 * screen_scaling) as usize;

    let width = image.width() as f32;
    let height = image.height() as f32;
    let most_width = PIXEL_REWARD_WIDTH * screen_scaling;
    let most_left = width / 2.0 - most_width / 2.0;
    // Most Top = pixleRewardYDisplay - pixleRewardHeight + pixelRewardLineHeight
    //                   (316          -        235        +       44)    *    1.1    =    137
    let most_top = height / 2.0
        - ((PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT + PIXEL_REWARD_LINE_HEIGHT)
            * screen_scaling);
    let most_bot =
        height / 2.0 - ((PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT) * screen_scaling * 0.5);

    let prefilter = image.crop_imm(
        most_left as u32,
        most_top as u32,
        most_width as u32,
        (most_bot - most_top) as u32,
    );
    let mut prefilter_draw = prefilter.clone().into_rgb8();
    // prefilter.save("prefilter.png").unwrap();

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
    let mut lowest_weight = 0.0;
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

        // debug!("");
        // debug!("i: {}", i);
        // debug!("y_from_top: {}", y_from_top);
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

        if scaling <= 0.0 || lowest_weight > perc_weights[i] {
            scaling = scale as f32;
            lowest_weight = perc_weights[i];
        }
    }

    debug!("Scaling: {}", scaling);

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

    debug!("top_five: {:?}", top_five);
    scaling = top_five[4] as f32 + 50.0;
    debug!("scaling: {:?}", top_five);

    scaling /= 100.0;
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

    let crop_width = PIXEL_REWARD_WIDTH * screen_scaling * high_scaling;
    let crop_left = prefilter.width() as f32 / 2.0 - crop_width / 2.0;
    let crop_top = height / 2.0
        - (PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT + PIXEL_REWARD_LINE_HEIGHT)
            * screen_scaling
            * high_scaling;
    let crop_bot =
        height / 2.0 - (PIXEL_REWARD_YDISPLAY - PIXEL_REWARD_HEIGHT) * screen_scaling * low_scaling;
    let crop_hei = crop_bot - crop_top;
    let crop_top = crop_top - most_top;

    let partial_screenshot = DynamicImage::ImageRgb8(prefilter.into_rgb8()).crop_imm(
        crop_left as u32,
        crop_top as u32,
        crop_width as u32,
        crop_hei as u32,
    );

    // Draw top 5
    for (i, y) in top_five.iter().enumerate() {
        for x in 0..prefilter_draw.width() {
            prefilter_draw.put_pixel(x, *y as u32, Rgb([255 - i as u8 * 50, 0, 0]));
        }
    }
    // Draw histogram
    for (y, row) in rows.iter().enumerate() {
        for x in 0..*row {
            prefilter_draw.put_pixel(x as u32, y as u32, Rgb([0, 255, 0]));
        }
    }

    // prefilter_draw.save("prefilter.png").unwrap();

    // partial_screenshot.save("partial_screenshot.png").unwrap();

    filter_and_separate_parts_from_part_box(partial_screenshot, theme)
}

/// Filters and separates individual parts from a reward box
///
/// This function:
/// 1. Converts the image to black and white based on theme colors
/// 2. Uses cosine analysis to detect reward spacing
/// 3. Determines if there are 3 or 4 rewards
/// 4. Splits the image into individual reward sections
pub fn filter_and_separate_parts_from_part_box(
    image: DynamicImage,
    theme: Theme,
) -> Vec<DynamicImage> {
    let mut filtered = image.into_rgb8();

    let mut _weight = 0.0;
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

        // debug!("{}", cosine_thing);

        let this_weight = cosine_thing * count as f32;
        _weight += this_weight;

        if cosine < 0.0 {
            total_even -= this_weight;
        } else if cosine > 0.0 {
            total_odd += this_weight;
        }
    }

    // filtered
    //     .save("filtered.png")
    //     .expect("Failed to write filtered image");

    if total_even == 0.0 && total_odd == 0.0 {
        return vec![];
    }

    let _total = total_even + total_odd;
    // debug!("Even: {}", total_even / total);
    // debug!("Odd: {}", total_odd / total);

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
        // cropped
        //     .save(format!("part-{}.png", i))
        //     .expect("Failed to write image");
        images.push(cropped);
    }

    images
}

lazy_static! {
    pub static ref OCR: Mutex<Option<Tesseract>> = Mutex::new(Some(
        Tesseract::new(None, Some("eng")).expect("Could not initialize Tesseract")
    ));
}

/// Processes a reward screen image and returns the names of all rewards
///
/// This function:
/// 1. Detects the theme if not provided
/// 2. Extracts individual reward parts
/// 3. Performs OCR on each part
/// 4. Returns a vector of reward names
pub fn reward_image_to_reward_names(image: DynamicImage, theme: Option<Theme>) -> Vec<String> {
    let theme = theme.unwrap_or_else(|| detect_theme(&image));
    let parts = extract_parts(&image, theme);
    debug!("Extracted part images");

    parts
        .iter()
        .map(|image| image_to_string(&mut OCR.lock().unwrap(), image))
        .collect()
}

/// Processes a single part image and returns its name
///
/// # Arguments
/// * `image` - Source image
/// * `theme` - Optional theme for color processing
/// * `sel_size` - Selection dimensions (width, height)
/// * `sel_pos` - Selection position (x, y)
/// * `brightness` - OCR brightness adjustment (-255 to 255)
/// * `contrast` - OCR contrast adjustment (0.0 to 10.0)
pub fn part_image_to_part_name(
    image: DynamicImage,
    _theme: Option<Theme>,
    sel_size: (i32, i32),
    sel_pos: (i32, i32),
    brightness: i32,
    contrast: f32,
) -> String {
    let part = extract_part(&image, sel_size, sel_pos, brightness, contrast);
    debug!("Extracted part image");

    let text = image_to_string(&mut OCR.lock().unwrap(), &part);
    text
}
