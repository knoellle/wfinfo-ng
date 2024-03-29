use std::ops::Range;

use image::Rgb;
use ordered_float::OrderedFloat;
use palette::{FromColor, Hsl, RgbHue, Srgb};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub struct HslRange<T> {
    pub hue: Range<T>,
    pub saturation: Range<T>,
    pub lightness: Range<T>,
}

impl HslRange<OrderedFloat<f32>> {
    fn get_average(&self) -> Hsl {
        Hsl::from_components((
            RgbHue::from_degrees(((self.hue.start + self.hue.end) / 2.0).0),
            ((self.saturation.start + self.saturation.end) / 2.0).0,
            ((self.lightness.start + self.lightness.end) / 2.0).0,
        ))
    }
}

impl HslRange<f32> {
    pub fn to_ordered(&self) -> HslRange<OrderedFloat<f32>> {
        HslRange {
            hue: OrderedFloat(self.hue.start)..OrderedFloat(self.hue.end),
            saturation: OrderedFloat(self.saturation.start)..OrderedFloat(self.saturation.end),
            lightness: OrderedFloat(self.lightness.start)..OrderedFloat(self.lightness.end),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Theme {
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
    Custom(HslRange<OrderedFloat<f32>>),
}

pub fn color_difference(colors: (Hsl, Hsl)) -> f32 {
    let rgb0 = Srgb::from_color(colors.0);
    let rgb1 = Srgb::from_color(colors.1);
    ((rgb0.red - rgb1.red).abs() + (rgb0.green - rgb1.green).abs() + (rgb0.blue - rgb1.blue).abs())
        * 255.0
}

impl Theme {
    pub fn closest_from_color(color: Rgb<u8>) -> (Theme, f32) {
        let rgb = Srgb::from_components((
            color.0[0] as f32 / 255.0,
            color.0[1] as f32 / 255.0,
            color.0[2] as f32 / 255.0,
        ));
        let hsl = Hsl::from_color(rgb);
        Self::iter()
            .map(|theme| (theme.clone(), color_difference((theme.primary(), hsl))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
    }

    pub fn iter() -> std::slice::Iter<'static, Theme> {
        [
            Self::Vitruvian,
            Self::Stalker,
            Self::Baruuk,
            Self::Corpus,
            Self::Fortuna,
            Self::Grineer,
            Self::Lotus,
            Self::Nidus,
            Self::Orokin,
            Self::Tenno,
            Self::HighContrast,
            Self::Legacy,
            Self::Equinox,
            Self::DarkLotus,
            Self::Zephyr,
        ]
        .iter()
    }

    pub fn threshold_filter(&self, color: Rgb<u8>) -> bool {
        let rgb = Srgb::from_components((
            color.0[0] as f32 / 255.0,
            color.0[1] as f32 / 255.0,
            color.0[2] as f32 / 255.0,
        ));
        let test = Hsl::from_color(rgb);

        let primary = self.primary();
        let secondary = self.secondary();

        match self {
            Theme::Equinox => test.saturation <= 0.2 && test.lightness >= 0.55,
            Theme::Stalker => {
                (0.61..1.00).contains(&test.saturation)
                    && (0.25..0.65).contains(&test.lightness)
                    && (-10.0..5.0).contains(&test.hue.into_degrees())
            }
            Theme::HighContrast => {
                test.saturation >= 0.60
                    && (0.23..0.45).contains(&test.lightness)
                    && (-160.0..-145.0).contains(&test.hue.into_degrees())
            }
            Theme::Custom(range) => {
                range.hue.contains(&OrderedFloat(test.hue.into_degrees()))
                    && range.saturation.contains(&OrderedFloat(test.saturation))
                    && range.lightness.contains(&OrderedFloat(test.lightness))
            }
            _ => {
                color_difference((primary, test)) < 0.2 || color_difference((secondary, test)) < 0.2
            }
        }

        // hsv.hue.abs_diff_eq(&primary.hue, 4.0) && hsv.saturation >= 0.25 && hsv.value >= 0.42

        // match self {
        //     Theme::Vitruvian => {
        //         test.hue.abs_diff_eq(&primary.hue, 4.0)
        //             && test.saturation >= 0.25
        //             && test.lightness >= 0.42
        //     }
        //     Theme::Stalker => {
        //         test.hue.abs_diff_eq(&primary.hue, 4.0) && test.saturation >= 0.55
        //             || test.hue.abs_diff_eq(&secondary.hue, 4.0)
        //                 && test.saturation >= 0.66
        //                 && test.lightness >= 0.25
        //     }
        //     Theme::Baruuk => {
        //         test.hue.abs_diff_eq(&primary.hue, 2.0)
        //             && test.saturation > 0.25
        //             && test.lightness > 0.5
        //     }
        //     Theme::Corpus => {
        //         test.hue.abs_diff_eq(&primary.hue, 3.0)
        //             && test.saturation >= 0.35
        //             && test.lightness >= 0.42
        //     }
        //     Theme::Fortuna => {
        //         test.hue.abs_diff_eq(&primary.hue, 3.0) && test.lightness >= 0.35
        //             || test.hue.abs_diff_eq(&secondary.hue, 4.0)
        //                 && test.saturation >= 0.2
        //                 && test.lightness >= 0.15
        //     }
        //     Theme::Grineer => todo!(),
        //     Theme::Lotus => {
        //         test.hue.abs_diff_eq(&primary.hue, 5.0)
        //             && test.saturation >= 0.65
        //             && primary.lightness.abs_diff_eq(&test.lightness, 0.1)
        //     }
        //     Theme::Nidus => todo!(),
        //     Theme::Orokin => todo!(),
        //     Theme::Tenno => todo!(),
        //     Theme::HighContrast => todo!(),
        //     Theme::Legacy => todo!(),
        //     Theme::Equinox => todo!(),
        //     Theme::DarkLotus => todo!(),
        //     Theme::Zephyr => todo!(),
        // }
    }

    pub fn primary(&self) -> Hsl {
        let components = match self {
            Theme::Vitruvian => (190, 169, 102),
            Theme::Stalker => (153, 31, 35),
            Theme::Baruuk => (238, 193, 105),
            Theme::Corpus => (35, 201, 245),
            Theme::Fortuna => (57, 105, 192),
            Theme::Grineer => (255, 189, 102),
            Theme::Lotus => (36, 184, 242),
            Theme::Nidus => (140, 38, 92),
            Theme::Orokin => (20, 41, 29),
            Theme::Tenno => (9, 78, 106),
            Theme::HighContrast => (2, 127, 217),
            Theme::Legacy => (255, 255, 255),
            Theme::Equinox => (158, 159, 167),
            Theme::DarkLotus => (140, 119, 147),
            Theme::Zephyr => (253, 132, 2),
            Theme::Custom(range) => return range.get_average(),
        };

        let components = (
            components.0 as f32 / 255.0,
            components.1 as f32 / 255.0,
            components.2 as f32 / 255.0,
        );
        Hsl::from_color(Srgb::from_components(components))
    }

    pub fn secondary(&self) -> Hsl {
        let components = match self {
            Theme::Vitruvian => (245, 227, 173),
            Theme::Stalker => (255, 61, 51),
            Theme::Baruuk => (236, 211, 162),
            Theme::Corpus => (111, 229, 253),
            Theme::Fortuna => (255, 115, 230),
            Theme::Grineer => (255, 224, 153),
            Theme::Lotus => (255, 241, 191),
            Theme::Nidus => (245, 73, 93),
            Theme::Orokin => (178, 125, 5),
            Theme::Tenno => (6, 106, 74),
            Theme::HighContrast => (255, 255, 0),
            Theme::Legacy => (232, 213, 93),
            Theme::Equinox => (232, 227, 227),
            Theme::DarkLotus => (189, 169, 237),
            Theme::Zephyr => (255, 53, 0),
            Theme::Custom(range) => return range.get_average(),
        };

        let components = (
            components.0 as f32 / 255.0,
            components.1 as f32 / 255.0,
            components.2 as f32 / 255.0,
        );
        Hsl::from_color(Srgb::from_components(components))
    }
}
