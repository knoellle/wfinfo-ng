use image::{GrayImage, Luma};
use imageproc::{
    definitions::Image,
    drawing::draw_filled_rect_mut,
    map::map_colors,
    rect::Rect,
    template_matching::{find_extremes, match_template, MatchTemplateMethod},
};

pub fn image_to_string(
    image: &GrayImage,
    templates: &[(char, GrayImage)],
    letter_threshold: f32,
) -> (String, f32, (u32, u32)) {
    let mut image = image.clone();
    let mut result = String::new();
    while let Some((letter, _confidence, position)) =
        best_letter(&image, templates, letter_threshold)
    {
        draw_filled_rect_mut(
            &mut image,
            Rect::at(position.0 as i32, position.1 as i32)
                .of_size(templates[0].1.width(), templates[0].1.height()),
            Luma([127]),
        );
        image.save("matches.png");
        result.push(letter);
    }

    (result, 0.0, (0, 0))
}

/// Convert an f32-valued image to a 8 bit depth, covering the whole
/// available intensity range.
fn convert_to_gray_image(image: &Image<Luma<f32>>) -> GrayImage {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;

    for p in image.iter() {
        lo = if *p < lo { *p } else { lo };
        hi = if *p > hi { *p } else { hi };
    }

    let range = hi - lo;
    let scale = |x| (255.0 * (x - lo) / range) as u8;
    map_colors(image, |p| Luma([scale(p[0])]))
}

pub fn best_letter(
    image: &GrayImage,
    templates: &[(char, GrayImage)],
    letter_threshold: f32,
) -> Option<(char, f32, (u32, u32))> {
    let method = MatchTemplateMethod::SumOfSquaredErrorsNormalized;
    let mut matches = templates
        .iter()
        .map(|(letter, template)| {
            let matches = match_template(image, template, method);
            // convert_to_gray_image(&matches).save("matches.png").unwrap();
            (letter.clone(), find_extremes(&matches))
        })
        .collect::<Vec<_>>();
    // find the best match
    matches.sort_by(|a, b| a.1.min_value.partial_cmp(&b.1.min_value).unwrap());
    let (letter, extreme) = dbg!(matches[0].clone());
    (extreme.min_value < letter_threshold).then_some((
        letter,
        extreme.min_value,
        extreme.min_value_location,
    ))
}
