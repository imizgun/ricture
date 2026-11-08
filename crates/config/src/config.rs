use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;
use validator::{Validate, ValidationError};

static HEX_COLOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^#[0-9a-f]{6}([0-9a-f]{2})?$").unwrap());

#[derive(Deserialize, Serialize, Validate)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[validate(nested)]
    pub general: ConfigGeneral,
    #[validate(nested)]
    pub appearance: ConfigAppearance,
    #[validate(nested)]
    pub tools: ConfigTools,
}

#[derive(Deserialize, Serialize, Validate)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct ConfigGeneral {
    #[validate(custom(function = "validate_save_path"))]
    pub save_path: String,

    pub enable_notifications: bool
}

const DEFAULT_RECT_COLOR: &'static str = "#ffffff";
const DEFAULT_BRUSH_COLOR: &'static str = "#ff3b30";
const DEFAULT_BRUSH_WIDTH: u32 = 4;

#[derive(Deserialize, Serialize, Validate)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct ConfigAppearance {
    #[validate(regex(
        path = *HEX_COLOR_RE,
        message = "invalid value for 'rect_color': valid value is '#rrggbb' or '#rrggbbaa'."
    ))]
    pub rect_color: String,
}

#[derive(Default, Deserialize, Serialize, Validate)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct ConfigTools {
    #[validate(nested)]
    pub brush: ConfigBrush,
}

#[derive(Deserialize, Serialize, Validate)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct ConfigBrush {
    #[validate(regex(
        path = *HEX_COLOR_RE,
        message = "invalid value for 'tools.brush.color': valid value is '#rrggbb' or '#rrggbbaa'."
    ))]
    pub color: String,

    /// Logical pixels. Values above the brush's maximum are limited to it.
    #[validate(range(min = 1, message = "invalid value for 'tools.brush.width': must be at least 1."))]
    pub width: u32,
}

fn validate_save_path(save_path: &str) -> Result<(), ValidationError> {
    if let Some(parent) = std::path::Path::new(save_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(ValidationError::new("missing_directory").with_message(
                format!(
                    "invalid value for 'save_path': directory '{}' does not exist.",
                    parent.display()
                )
                .into(),
            ));
        }
    }
    Ok(())
}

impl Default for ConfigGeneral {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        ConfigGeneral {
            save_path: format!("{home}/Pictures/Screenshots"),
            enable_notifications: true
        }
    }
}

impl Default for ConfigAppearance {
    fn default() -> Self {
        ConfigAppearance {
            rect_color: DEFAULT_RECT_COLOR.to_string(),
        }
    }
}

impl Default for ConfigBrush {
    fn default() -> Self {
        ConfigBrush {
            color: DEFAULT_BRUSH_COLOR.to_string(),
            width: DEFAULT_BRUSH_WIDTH,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            general: ConfigGeneral::default(),
            appearance: ConfigAppearance::default(),
            tools: ConfigTools::default(),
        }
    }
}

impl Config {
    fn path() -> PathBuf {
        let config_home = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").expect("HOME not set")).join(".config")
            });
        config_home.join("ricture").join("config.toml")
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::path();

        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let default = Config::default();
            std::fs::write(&path, toml::to_string_pretty(&default)?)?;
            return Ok(default);
        }

        Ok(toml::from_str(&std::fs::read_to_string(&path)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_without_tools_section_uses_defaults() {
        let config: Config = toml::from_str("[appearance]\nrect_color = \"#000000\"\n").unwrap();
        assert_eq!(config.tools.brush.color, DEFAULT_BRUSH_COLOR);
        assert_eq!(config.tools.brush.width, DEFAULT_BRUSH_WIDTH);
    }

    #[test]
    fn brush_section_overrides_only_what_it_sets() {
        let config: Config = toml::from_str("[tools.brush]\nwidth = 9\n").unwrap();
        assert_eq!(config.tools.brush.color, DEFAULT_BRUSH_COLOR);
        assert_eq!(config.tools.brush.width, 9);
    }

    #[test]
    fn example_config_parses() {
        let config: Config = toml::from_str(include_str!("../../../examples/config.toml")).unwrap();
        assert!(config.tools.brush.validate().is_ok());
    }

    #[test]
    fn invalid_brush_values_are_rejected() {
        let brush = |color: &str, width| ConfigBrush { color: color.to_string(), width };
        assert!(brush("#ff0000", 1).validate().is_ok());
        assert!(brush("#ff000080", 1).validate().is_ok());
        assert!(brush("red", 1).validate().is_err());
        assert!(brush("#ff0000", 0).validate().is_err());
    }
}
