use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;
use validator::{Validate, ValidationError};

static RECT_COLOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^#[0-9a-f]{6}([0-9a-f]{2})?$").unwrap());

#[derive(Deserialize, Serialize, Validate)]
pub struct Config {
    #[validate(nested)]
    pub general: ConfigGeneral,
    #[validate(nested)]
    pub appearance: ConfigAppearance,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct ConfigGeneral {
    #[validate(custom(function = "validate_save_path"))]
    pub save_path: String,
}

const DEFAULT_RECT_COLOR: &'static str = "#ffffff";

#[derive(Deserialize, Serialize, Validate)]
pub struct ConfigAppearance {
    #[validate(regex(
        path = *RECT_COLOR_RE,
        message = "invalid value for 'rect_color': valid value is '#rrggbb' or '#rrggbbaa'."
    ))]
    pub rect_color: String,
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

impl Default for Config {
    fn default() -> Self {
        Config {
            general: ConfigGeneral::default(),
            appearance: ConfigAppearance::default(),
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
