use crate::validate::Validate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub general: ConfigGeneral,
    pub appearance: ConfigAppearance,
}

#[derive(Deserialize, Serialize)]
pub struct ConfigGeneral {
    pub save_path: String,
}

const DEFAULT_RECT_COLOR: &'static str = "#ffffff";

#[derive(Deserialize, Serialize)]
pub struct ConfigAppearance {
    pub rect_color: String,
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

impl Validate<Config> for Config {
    fn validate(&self) -> Result<(), String> {
        let g_err = self.general.validate();
        let a_err = self.appearance.validate();

        if let Err(e) = g_err {
            return Err(e);
        }
        if let Err(e) = a_err {
            return Err(e);
        }
        Ok(())
    }
}

impl Validate<ConfigAppearance> for ConfigAppearance {
    fn validate(&self) -> Result<(), String> {
        let re = Regex::new(r"(?i)^#[0-9a-f]{6}([0-9a-f]{2})?$").unwrap();

        if !re.is_match(&self.rect_color) {
            return Err(
                "invalid value for 'rect_color': valid value is '#rrggbb' or '#rrggbbaa'."
                    .to_string(),
            );
        }

        Ok(())
    }
}

impl Validate<ConfigGeneral> for ConfigGeneral {
    fn validate(&self) -> Result<(), String> {
        if let Some(parent) = std::path::Path::new(&self.save_path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(format!(
                    "invalid value for 'save_path': directory '{}' does not exist.",
                    parent.display()
                ));
            }
        }
        Ok(())
    }
}
