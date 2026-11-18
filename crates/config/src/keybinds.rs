//! Key bindings: which keys trigger which action.
//!
//! A bind is written as `Ctrl+Shift+Z`: zero or more modifiers (`ctrl`,
//! `shift`, `alt`, `super`) and one key, joined by `+` and case-insensitive.
//!
//! Keys are matched by *position*, not by the symbol the active layout
//! produces: a bind stores the evdev keycode the compositor reports, so
//! `Ctrl+C` keeps working on a Cyrillic (or Dvorak) layout. The names below
//! are therefore the labels of a US layout.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use validator::{Validate, ValidationError};

/// The modifiers a bind requires, all of which must match exactly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// The "super"/"win" key.
    pub logo: bool,
}

/// A key plus its modifiers, as reported by the compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub mods: Mods,
    /// evdev keycode (`linux/input-event-codes.h`).
    pub key: u32,
}

impl Chord {
    pub fn parse(spec: &str) -> Result<Self, String> {
        let parts: Vec<&str> = spec.split('+').map(str::trim).collect();
        let (name, modifiers) = parts.split_last().expect("split yields one part");

        let mut mods = Mods::default();
        for modifier in modifiers {
            match modifier.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => mods.ctrl = true,
                "shift" => mods.shift = true,
                "alt" => mods.alt = true,
                "super" | "logo" | "meta" | "win" => mods.logo = true,
                other => return Err(format!("unknown modifier '{other}'")),
            }
        }

        match key_code(name) {
            Some(key) => Ok(Chord { mods, key }),
            None if name.is_empty() => Err("no key named".to_string()),
            None => Err(format!("unknown key '{name}'")),
        }
    }
}

/// Canonical spelling, so what the UI shows matches what the config takes.
impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, name) in [
            (self.mods.ctrl, "Ctrl"),
            (self.mods.alt, "Alt"),
            (self.mods.logo, "Super"),
            (self.mods.shift, "Shift"),
        ] {
            if held {
                write!(f, "{name}+")?;
            }
        }
        match key_name(self.key) {
            Some(name) => write!(f, "{name}"),
            None => write!(f, "<{}>", self.key),
        }
    }
}

/// One or several keys for the same action: `"Ctrl+S"` or
/// `["Return", "Space"]`. An empty list leaves the action unbound.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Keys {
    One(String),
    Many(Vec<String>),
}

impl Default for Keys {
    fn default() -> Self {
        Keys::Many(Vec::new())
    }
}

impl From<&str> for Keys {
    fn from(spec: &str) -> Self {
        Keys::One(spec.to_string())
    }
}

impl Keys {
    pub fn many<'a>(specs: impl IntoIterator<Item = &'a str>) -> Self {
        Keys::Many(specs.into_iter().map(str::to_string).collect())
    }

    pub fn specs(&self) -> &[String] {
        match self {
            Keys::One(spec) => std::slice::from_ref(spec),
            Keys::Many(specs) => specs,
        }
    }
}

/// The configurable binds. Tool keys live in [`ConfigKeybinds::tools`] because
/// which tools exist is up to `ricture-tools`, not to this crate.
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct ConfigKeybinds {
    /// Close the overlay, keeping nothing.
    #[validate(custom(function = "validate_keys"))]
    pub cancel: Keys,
    #[validate(custom(function = "validate_keys"))]
    pub copy: Keys,
    #[validate(custom(function = "validate_keys"))]
    pub save: Keys,
    #[validate(custom(function = "validate_keys"))]
    pub undo: Keys,
    /// Leave the active tool and select the area again.
    #[validate(custom(function = "validate_keys"))]
    pub select: Keys,

    /// Per tool, by the id in its `ToolMeta` (`brush`, `arrow`, `line`...).
    /// Tools left out keep their default: the digit of their place in the
    /// toolbar, `1` to `9`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[validate(custom(function = "validate_tool_keys"))]
    pub tools: BTreeMap<String, Keys>,
}

impl Default for ConfigKeybinds {
    fn default() -> Self {
        ConfigKeybinds {
            cancel: "Escape".into(),
            copy: Keys::many(["Return", "Space", "Ctrl+C"]),
            save: "Ctrl+S".into(),
            undo: "Ctrl+Z".into(),
            select: "0".into(),
            tools: BTreeMap::new(),
        }
    }
}

fn validate_keys(keys: &Keys) -> Result<(), ValidationError> {
    for spec in keys.specs() {
        if let Err(err) = Chord::parse(spec) {
            return Err(ValidationError::new("invalid_keybind")
                .with_message(format!("invalid keybind '{spec}': {err}").into()));
        }
    }
    Ok(())
}

fn validate_tool_keys(tools: &BTreeMap<String, Keys>) -> Result<(), ValidationError> {
    for keys in tools.values() {
        validate_keys(keys)?;
    }
    Ok(())
}

/// Rows of a US keyboard with the evdev code of their first key; the keys of
/// a row are numbered consecutively.
const ROWS: [(&str, u32); 4] = [
    ("1234567890", 2),
    ("qwertyuiop", 16),
    ("asdfghjkl", 30),
    ("zxcvbnm", 44),
];

/// Everything that isn't a plain letter or digit. The first spelling of a
/// code is the one [`Chord::Display`] uses.
const NAMED: [(&str, u32); 40] = [
    ("Escape", 1),
    ("Esc", 1),
    ("Minus", 12),
    ("Equal", 13),
    ("Backspace", 14),
    ("Tab", 15),
    ("BracketLeft", 26),
    ("BracketRight", 27),
    ("Return", 28),
    ("Enter", 28),
    ("Semicolon", 39),
    ("Apostrophe", 40),
    ("Grave", 41),
    ("Backslash", 43),
    ("Comma", 51),
    ("Period", 52),
    ("Slash", 53),
    ("Space", 57),
    ("F1", 59),
    ("F2", 60),
    ("F3", 61),
    ("F4", 62),
    ("F5", 63),
    ("F6", 64),
    ("F7", 65),
    ("F8", 66),
    ("F9", 67),
    ("F10", 68),
    ("F11", 87),
    ("F12", 88),
    ("Home", 102),
    ("Up", 103),
    ("PageUp", 104),
    ("Left", 105),
    ("Right", 106),
    ("End", 107),
    ("Down", 108),
    ("PageDown", 109),
    ("Insert", 110),
    ("Delete", 111),
];

fn key_code(name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    if let [ch] = lower.as_bytes() {
        let ch = *ch as char;
        for (row, first) in ROWS {
            if let Some(offset) = row.find(ch) {
                return Some(first + offset as u32);
            }
        }
    }
    NAMED
        .iter()
        .find(|(named, _)| named.eq_ignore_ascii_case(name))
        .map(|&(_, code)| code)
}

fn key_name(code: u32) -> Option<String> {
    for (row, first) in ROWS {
        if let Some(offset) = code.checked_sub(first).filter(|&o| (o as usize) < row.len()) {
            return Some(row[offset as usize..][..1].to_uppercase());
        }
    }
    NAMED
        .iter()
        .find(|&&(_, named)| named == code)
        .map(|&(name, _)| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(spec: &str) -> Chord {
        Chord::parse(spec).expect("valid bind")
    }

    #[test]
    fn keys_are_matched_by_position_on_a_us_layout() {
        assert_eq!(chord("c").key, 46);
        assert_eq!(chord("1").key, 2);
        assert_eq!(chord("0").key, 11);
        assert_eq!(chord("Escape").key, 1);
        assert_eq!(chord("Space").key, 57);
        assert_eq!(chord("F12").key, 88);
    }

    #[test]
    fn modifiers_and_key_names_ignore_case_and_spacing() {
        assert_eq!(chord("CTRL + z"), chord("control+Z"));
        assert_eq!(
            chord("super+alt+shift+Return").mods,
            Mods { ctrl: false, shift: true, alt: true, logo: true }
        );
    }

    #[test]
    fn modifiers_must_match_exactly() {
        assert_ne!(chord("Ctrl+C"), chord("Ctrl+Shift+C"));
        assert_ne!(chord("c").mods, chord("Ctrl+C").mods);
    }

    #[test]
    fn nonsense_binds_are_rejected() {
        assert!(Chord::parse("").is_err());
        assert!(Chord::parse("Ctrl+").is_err());
        assert!(Chord::parse("Hyper+C").is_err());
        assert!(Chord::parse("F13").is_err());
        assert!(Chord::parse("ctrl").is_err(), "a lone modifier is not a key");
    }

    #[test]
    fn binds_are_displayed_the_way_they_are_written() {
        for spec in ["Ctrl+C", "1", "Escape", "Ctrl+Alt+Super+Shift+Delete", "F5", "Shift+A"] {
            assert_eq!(chord(spec).to_string(), spec);
        }
        assert_eq!(chord("enter").to_string(), "Return", "aliases fold into one name");
    }

    #[test]
    fn keys_take_a_single_bind_or_a_list() {
        #[derive(Deserialize)]
        struct Wrapper {
            keys: Keys,
        }

        let one: Wrapper = toml::from_str("keys = \"Ctrl+S\"").unwrap();
        assert_eq!(one.keys.specs(), ["Ctrl+S"]);
        let many: Wrapper = toml::from_str("keys = [\"Return\", \"Space\"]").unwrap();
        assert_eq!(many.keys.specs(), ["Return", "Space"]);
        let none: Wrapper = toml::from_str("keys = []").unwrap();
        assert!(none.keys.specs().is_empty(), "an empty list unbinds the action");
    }

    #[test]
    fn the_defaults_are_valid() {
        assert!(ConfigKeybinds::default().validate().is_ok());
    }

    #[test]
    fn invalid_binds_fail_validation() {
        let binds = ConfigKeybinds { undo: "Ctrl+Nonsense".into(), ..Default::default() };
        assert!(binds.validate().is_err());

        let mut binds = ConfigKeybinds::default();
        binds.tools.insert("brush".to_string(), "Shift+Click".into());
        assert!(binds.validate().is_err());
    }
}
