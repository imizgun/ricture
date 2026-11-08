use tiny_skia::Color;

/// A color as the picker edits it. Every component is in `0.0..=1.0`.
///
/// RGB can't stand in for this: at zero saturation or value it loses the hue,
/// and the hue slider would jump back to red.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Hsv {
    pub(crate) hue: f32,
    pub(crate) saturation: f32,
    pub(crate) value: f32,
}

impl Hsv {
    pub(crate) fn from_color(color: Color) -> Self {
        let (r, g, b) = (color.red(), color.green(), color.blue());
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        let sector = if delta == 0.0 {
            0.0
        } else if max == r {
            ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };
        Self {
            hue: sector / 6.0,
            saturation: if max == 0.0 { 0.0 } else { delta / max },
            value: max,
        }
    }

    pub(crate) fn to_color(self, alpha: u8) -> Color {
        let Self { hue, saturation: s, value: v } = self;
        let sector = hue.rem_euclid(1.0) * 6.0;
        let f = sector.fract();
        let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
        let (r, g, b) = match sector as u8 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        let byte = |channel: f32| (channel * 255.0).round() as u8;
        Color::from_rgba8(byte(r), byte(g), byte(b), alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::from_rgba8(r, g, b, 255)
    }

    #[test]
    fn primaries_have_the_expected_hue() {
        assert_eq!(Hsv::from_color(rgb(255, 0, 0)).hue, 0.0);
        assert!((Hsv::from_color(rgb(0, 255, 0)).hue - 1.0 / 3.0).abs() < 1e-6);
        assert!((Hsv::from_color(rgb(0, 0, 255)).hue - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn round_trips_through_rgb() {
        for color in [
            rgb(0xff, 0x3b, 0x30),
            rgb(0x34, 0xc7, 0x59),
            rgb(0x0a, 0x84, 0xff),
            rgb(0xaf, 0x52, 0xde),
            rgb(0, 0, 0),
            rgb(255, 255, 255),
            rgb(128, 128, 128),
        ] {
            assert_eq!(Hsv::from_color(color).to_color(255), color);
        }
    }

    #[test]
    fn full_hue_wraps_to_red() {
        let hsv = Hsv { hue: 1.0, saturation: 1.0, value: 1.0 };
        assert_eq!(hsv.to_color(255), rgb(255, 0, 0));
    }
}
