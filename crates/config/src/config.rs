use serde::Deserialize;
use regex::Regex;
use crate::validate::Validate;

#[derive(Deserialize)]
pub struct Config {
    pub general: ConfigGeneral,
    pub appearance: ConfigAppearance
}

#[derive(Deserialize)]
pub struct ConfigGeneral {
    pub save_path: String,
}

#[derive(Deserialize)]
pub struct ConfigAppearance {
    pub rect_color: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config = toml::from_str(include_str!("../../../examples/config.toml"))?;
        Ok(config)
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
        let re = Regex::new(r"(?i)^#[0-9a-f]{8}$").unwrap();

        if !re.is_match(&self.rect_color) {
            return Err("invalid value for 'rect_color': valid value is 'rrggbbaa'.".to_string())
        }

        Ok(())
    }
}

impl Validate<ConfigGeneral> for ConfigGeneral {
    fn validate(&self) -> Result<(), String> {
        if let Some(parent) = std::path::Path::new(&self.save_path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(format!("invalid value for 'save_path': directory '{}' does not exist.", parent.display()));
            }
        }
        Ok(())
    }
}