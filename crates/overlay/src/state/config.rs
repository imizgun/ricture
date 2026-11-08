use ricture_config::config::Config;
use tiny_skia::Color;

pub struct AppConfig {
    pub(crate) rect_color: Color,
    pub(crate) brush_color: Color,
    /// Logical px.
    pub(crate) brush_width: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Config::default().into()
    }
}

impl From<Config> for AppConfig {
    fn from(config: Config) -> Self {
        AppConfig {
            rect_color: parse_rrggbbaa(&config.appearance.rect_color),
            brush_color: parse_rrggbbaa(&config.tools.brush.color),
            brush_width: config.tools.brush.width as f32,
        }
    }
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
