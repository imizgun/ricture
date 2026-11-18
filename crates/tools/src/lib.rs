//! Annotation tools.
//!
//! The overlay only knows the abstractions in [`tool`] and the [`Document`]
//! that stores finished annotations; concrete tools are plain implementations
//! of [`Tool`] handed to it at startup (see [`default_tools`]). Adding a tool
//! means implementing the traits and registering it there — the overlay and
//! its UI pick it up without changes.
//!
//! All geometry is in screenshot (physical) pixels.

mod brush;
mod document;
mod shapes;
mod tool;

pub use brush::Brush;
pub use document::Document;
pub use shapes::{Arrow, Line};
pub use tool::{Annotation, Gesture, Setting, Style, Tool, ToolMeta};

/// Re-exported so tool implementations and the overlay agree on the version.
pub use tiny_skia;

/// The built-in tool set, in toolbar order. The position also decides a
/// tool's default keybind (`1`, `2`, `3`...), see `ricture-config`'s
/// `keybinds`.
pub fn default_tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(Brush), Box::new(Arrow), Box::new(Line)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn tool_ids_are_unique() {
        let tools = default_tools();
        let ids: BTreeSet<&str> = tools.iter().map(|tool| tool.meta().id).collect();
        assert_eq!(ids.len(), tools.len(), "ids key the config, so they must differ");
    }
}
