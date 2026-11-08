use ricture_config::config::{Config, DEFAULT_BRUSH_COLOR, DEFAULT_BRUSH_WIDTH};
use ricture_config::last_used::LastUsed;
use tiny_skia::Color;

pub struct AppConfig {
    pub(crate) rect_color: Color,
    pub(crate) brush_color: Color,
    /// Logical px.
    pub(crate) brush_width: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new(Config::default(), LastUsed::default())
    }
}

impl AppConfig {
    /// A value set in the config always wins; otherwise the brush starts
    /// where the last session left it.
    pub fn new(config: Config, last_used: LastUsed) -> Self {
        let brush = config.tools.brush;
        let color = brush.color.or(last_used.brush.color);
        let width = brush.width.or(last_used.brush.width);
        AppConfig {
            rect_color: parse_rrggbbaa(&config.appearance.rect_color),
            brush_color: parse_rrggbbaa(color.as_deref().unwrap_or(DEFAULT_BRUSH_COLOR)),
            brush_width: width.unwrap_or(DEFAULT_BRUSH_WIDTH) as f32,
        }
    }
}

pub(crate) fn format_rrggbbaa(color: Color) -> String {
    let c = color.to_color_u8();
    format!("#{:02x}{:02x}{:02x}{:02x}", c.red(), c.green(), c.blue(), c.alpha())
}

/// Parses `#rrggbb` (alpha defaults to 100%) or `#rrggbbaa` — both validated
/// by `ricture-config`'s `Validate` impl.
fn parse_rrggbbaa(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).expect("hex color validated by ricture-config");
    let (rgb, a) = if hex.len() == 6 {
        (value, 0xff)
    } else {
        (value >> 8, value & 0xff)
    };
    Color::from_rgba8(
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
        a as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ricture_config::last_used::LastBrush;

    fn config(color: Option<&str>, width: Option<u32>) -> Config {
        let mut config = Config::default();
        config.tools.brush.color = color.map(str::to_string);
        config.tools.brush.width = width;
        config
    }

    fn last_used(color: Option<&str>, width: Option<u32>) -> LastUsed {
        LastUsed {
            brush: LastBrush { color: color.map(str::to_string), width },
        }
    }

    #[test]
    fn config_wins_over_last_used() {
        let app = AppConfig::new(
            config(Some("#010203"), Some(9)),
            last_used(Some("#040506"), Some(5)),
        );
        assert_eq!(app.brush_color, Color::from_rgba8(1, 2, 3, 255));
        assert_eq!(app.brush_width, 9.0);
    }

    #[test]
    fn last_used_fills_in_what_the_config_leaves_out() {
        let app = AppConfig::new(config(None, Some(9)), last_used(Some("#040506"), Some(5)));
        assert_eq!(app.brush_color, Color::from_rgba8(4, 5, 6, 255));
        assert_eq!(app.brush_width, 9.0);
    }

    #[test]
    fn built_in_defaults_apply_on_first_run() {
        let app = AppConfig::new(config(None, None), LastUsed::default());
        assert_eq!(app.brush_color, parse_rrggbbaa(DEFAULT_BRUSH_COLOR));
        assert_eq!(app.brush_width, DEFAULT_BRUSH_WIDTH as f32);
    }

    #[test]
    fn colors_survive_formatting() {
        let color = Color::from_rgba8(0x12, 0xab, 0x00, 0x80);
        assert_eq!(format_rrggbbaa(color), "#12ab0080");
        assert_eq!(parse_rrggbbaa(&format_rrggbbaa(color)), color);
    }
}
