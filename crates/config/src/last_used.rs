use crate::config::HEX_COLOR_RE;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};
use validator::Validate;

/// Tool settings from the last confirmed screenshot. Kept apart from the
/// config, which belongs to the user and may be read-only.
#[derive(Default, Deserialize, Serialize, Validate)]
#[serde(default)]
pub struct LastUsed {
    #[validate(nested)]
    pub brush: LastBrush,
}

#[derive(Default, Deserialize, Serialize, Validate)]
#[serde(default)]
pub struct LastBrush {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(regex(path = *HEX_COLOR_RE))]
    pub color: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1))]
    pub width: Option<u32>,
}

impl LastUsed {
    /// Never fails: a missing or damaged file only means nothing is
    /// remembered, which must not stand in the way of a screenshot.
    pub fn load() -> Self {
        Self::path()
            .and_then(|path| Self::read(&path))
            .unwrap_or_else(|err| {
                eprintln!("warning: ignoring remembered tool settings: {err}");
                Self::default()
            })
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        self.write(&Self::path()?)
    }

    fn path() -> Result<PathBuf, Box<dyn Error>> {
        let state_home = match std::env::var("XDG_STATE_HOME") {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => PathBuf::from(std::env::var("HOME")?).join(".local/state"),
        };
        Ok(state_home.join("ricture").join("state.toml"))
    }

    fn read(path: &Path) -> Result<Self, Box<dyn Error>> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let last_used: Self = toml::from_str(&std::fs::read_to_string(path)?)?;
        last_used.validate()?;
        Ok(last_used)
    }

    fn write(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path in a directory of its own, which doesn't exist yet.
    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("ricture-last-used-{}-{name}", std::process::id()))
            .join("state.toml")
    }

    fn remove(path: &Path) {
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn missing_file_remembers_nothing() {
        let last_used = LastUsed::read(&temp_file("missing")).unwrap();
        assert!(last_used.brush.color.is_none());
        assert!(last_used.brush.width.is_none());
    }

    #[test]
    fn written_values_are_read_back() {
        let path = temp_file("round-trip");
        let saved = LastUsed {
            brush: LastBrush {
                color: Some("#12345680".to_string()),
                width: Some(7),
            },
        };
        saved.write(&path).unwrap();

        let loaded = LastUsed::read(&path).unwrap();
        remove(&path);
        assert_eq!(loaded.brush.color.as_deref(), Some("#12345680"));
        assert_eq!(loaded.brush.width, Some(7));
    }

    fn read_text(name: &str, text: &str) -> Result<LastUsed, Box<dyn Error>> {
        let path = temp_file(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        let result = LastUsed::read(&path);
        remove(&path);
        result
    }

    #[test]
    fn damaged_files_are_errors() {
        assert!(read_text("not-toml", "= nonsense").is_err());
        assert!(read_text("bad-color", "[brush]\ncolor = \"red\"\n").is_err());
        assert!(read_text("bad-width", "[brush]\nwidth = 0\n").is_err());
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        // A newer version may have written more than this one knows about.
        let loaded = read_text("unknown", "[brush]\nwidth = 3\nshape = \"round\"\n").unwrap();
        assert_eq!(loaded.brush.width, Some(3));
    }
}
