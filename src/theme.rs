use approx::AbsDiffEq;
use image::Rgb;
use palette::{FromColor, Hsv, Srgb};

#[derive(Debug, Clone, Copy)]
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
    Unknown,
}

impl Theme {
    pub fn threshold_filter(&self, color: Rgb<u8>) -> bool {
        let rgb = Srgb::from_components((
            color.0[0] as f32 / 255.0,
            color.0[1] as f32 / 255.0,
            color.0[2] as f32 / 255.0,
        ));
        let test = Hsv::from_color(rgb);

        // let primary = Hsv::from_color(Srgb::from_components((
        //     190.0 / 255.0,
        //     169.0 / 255.0,
        //     102.0 / 255.0,
        // )));
        let primary = self.primary();
        let secondary = self.secondary();

        // hsv.hue.abs_diff_eq(&primary.hue, 4.0) && hsv.saturation >= 0.25 && hsv.value >= 0.42

        match self {
            Theme::Vitruvian => {
                test.hue.abs_diff_eq(&primary.hue, 4.0)
                    && test.saturation >= 0.25
                    && test.value >= 0.42
            }
            Theme::Stalker => {
                test.hue.abs_diff_eq(&primary.hue, 4.0) && test.saturation >= 0.55
                    || test.hue.abs_diff_eq(&secondary.hue, 4.0)
                        && test.saturation >= 0.66
                        && test.value >= 0.25
            }
            Theme::Baruuk => {
                test.hue.abs_diff_eq(&primary.hue, 2.0)
                    && test.saturation > 0.25
                    && test.value > 0.5
            }
            Theme::Corpus => {
                test.hue.abs_diff_eq(&primary.hue, 3.0)
                    && test.saturation >= 0.35
                    && test.value >= 0.42
            }
            Theme::Fortuna => {
                test.hue.abs_diff_eq(&primary.hue, 3.0) && test.value >= 0.35
                    || test.hue.abs_diff_eq(&secondary.hue, 4.0) && test.value >= 0.15
            }
            Theme::Grineer => todo!(),
            Theme::Lotus => {
                test.hue.abs_diff_eq(&primary.hue, 5.0)
                    && test.saturation >= 0.65
                    && primary.value.abs_diff_eq(&test.value, 0.1)
            }
            Theme::Nidus => todo!(),
            Theme::Orokin => todo!(),
            Theme::Tenno => todo!(),
            Theme::HighContrast => todo!(),
            Theme::Legacy => todo!(),
            Theme::Equinox => todo!(),
            Theme::DarkLotus => todo!(),
            Theme::Zephyr => todo!(),
            Theme::Unknown => todo!(),
        }
    }

    pub fn primary(&self) -> Hsv {
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
            Theme::Unknown => (0, 0, 0),
        };

        let components = (
            components.0 as f32 / 255.0,
            components.1 as f32 / 255.0,
            components.2 as f32 / 255.0,
        );
        Hsv::from_color(Srgb::from_components(components))
    }

    pub fn secondary(&self) -> Hsv {
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
            Theme::Unknown => (0, 0, 0),
        };

        let components = (
            components.0 as f32 / 255.0,
            components.1 as f32 / 255.0,
            components.2 as f32 / 255.0,
        );
        Hsv::from_color(Srgb::from_components(components))
    }
}
