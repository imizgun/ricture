use ricture_config::keybinds::{Chord, ConfigKeybinds, Mods};

/// What a key does. The keyboard handler is the only thing that decides *when*
/// a binding may run (undo waits for a gesture to finish, for instance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Binding {
    Cancel,
    Copy,
    Save,
    Undo,
    /// Switch tool; `None` is Select mode.
    Tool(Option<usize>),
}

/// The configured binds resolved against the tools actually registered.
///
/// Lookup is by evdev keycode plus modifiers, so a bind follows the key's
/// position and survives a layout switch (see `ricture-config`'s `keybinds`).
pub(crate) struct Keymap {
    binds: Vec<(Chord, Binding)>,
    /// What the UI shows next to Select and each tool, in toolbar order;
    /// empty for anything left unbound.
    select_label: String,
    tool_labels: Vec<String>,
}

impl Keymap {
    /// `tool_ids` are the registered tools' [`ricture_tools::ToolMeta::id`]s,
    /// in toolbar order. A tool the config doesn't name gets the digit of its
    /// place in the toolbar.
    pub(crate) fn new(config: &ConfigKeybinds, tool_ids: &[&str]) -> Self {
        let mut keymap = Keymap {
            binds: Vec::new(),
            select_label: String::new(),
            tool_labels: vec![String::new(); tool_ids.len()],
        };

        for (keys, binding) in [
            (&config.cancel, Binding::Cancel),
            (&config.copy, Binding::Copy),
            (&config.save, Binding::Save),
            (&config.undo, Binding::Undo),
            (&config.select, Binding::Tool(None)),
        ] {
            keymap.bind(keys.specs(), binding, true);
        }

        // Configured tools first: one of them may well sit on the digit
        // another tool would only default to.
        for (index, id) in tool_ids.iter().enumerate() {
            if let Some(keys) = config.tools.get(*id) {
                keymap.bind(keys.specs(), Binding::Tool(Some(index)), true);
            }
        }
        for (index, id) in tool_ids.iter().enumerate() {
            if let Some(digit) = position_key(index).filter(|_| !config.tools.contains_key(*id)) {
                keymap.bind(&[digit], Binding::Tool(Some(index)), false);
            }
        }

        for id in config.tools.keys() {
            if !tool_ids.contains(&id.as_str()) {
                eprintln!("warning: keybinds.tools.{id}: no tool with that name");
            }
        }

        keymap
    }

    /// The binding for a key press, if any.
    pub(crate) fn lookup(&self, mods: Mods, key: u32) -> Option<Binding> {
        self.binds
            .iter()
            .find(|(chord, _)| chord.key == key && chord.mods == mods)
            .map(|&(_, binding)| binding)
    }

    pub(crate) fn select_label(&self) -> &str {
        &self.select_label
    }

    /// One label per registered tool, in toolbar order.
    pub(crate) fn tool_labels(&self) -> &[String] {
        &self.tool_labels
    }

    /// Adds `specs` for `binding`. The first bind of a key wins; `explicit`
    /// marks binds the user wrote, which are worth a warning when they clash.
    fn bind(&mut self, specs: &[String], binding: Binding, explicit: bool) {
        for spec in specs {
            let chord = match Chord::parse(spec) {
                Ok(chord) => chord,
                Err(err) => {
                    eprintln!("warning: ignoring keybind '{spec}': {err}");
                    continue;
                }
            };
            if self.binds.iter().any(|(bound, _)| *bound == chord) {
                if explicit {
                    eprintln!("warning: '{chord}' is bound twice; keeping the first binding");
                }
                continue;
            }
            self.binds.push((chord, binding));

            // The first bind of a tool is the one the UI advertises.
            if let Some(label) = self.label_mut(binding)
                && label.is_empty()
            {
                *label = chord.to_string();
            }
        }
    }

    fn label_mut(&mut self, binding: Binding) -> Option<&mut String> {
        match binding {
            Binding::Tool(None) => Some(&mut self.select_label),
            Binding::Tool(Some(index)) => self.tool_labels.get_mut(index),
            _ => None,
        }
    }
}

/// The default key for the tool in position `index`: `1`, `2`, ... `9`.
/// Further tools start out unbound.
fn position_key(index: usize) -> Option<String> {
    (index < 9).then(|| ((b'1' + index as u8) as char).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ricture_config::keybinds::Keys;

    const TOOLS: [&str; 3] = ["brush", "arrow", "line"];

    fn ctrl() -> Mods {
        Mods { ctrl: true, ..Default::default() }
    }

    fn keymap(config: ConfigKeybinds) -> Keymap {
        Keymap::new(&config, &TOOLS)
    }

    /// The evdev code of a key, the way the compositor reports it.
    fn code(spec: &str) -> u32 {
        Chord::parse(spec).unwrap().key
    }

    fn press(keymap: &Keymap, spec: &str) -> Option<Binding> {
        let chord = Chord::parse(spec).unwrap();
        keymap.lookup(chord.mods, chord.key)
    }

    #[test]
    fn default_binds_cover_the_documented_keys() {
        let keymap = keymap(ConfigKeybinds::default());
        assert_eq!(press(&keymap, "Escape"), Some(Binding::Cancel));
        assert_eq!(press(&keymap, "Return"), Some(Binding::Copy));
        assert_eq!(press(&keymap, "Space"), Some(Binding::Copy));
        assert_eq!(press(&keymap, "Ctrl+C"), Some(Binding::Copy));
        assert_eq!(press(&keymap, "Ctrl+S"), Some(Binding::Save));
        assert_eq!(press(&keymap, "Ctrl+Z"), Some(Binding::Undo));
        assert_eq!(press(&keymap, "0"), Some(Binding::Tool(None)));
    }

    #[test]
    fn tools_get_the_digit_of_their_place_in_the_toolbar() {
        let keymap = keymap(ConfigKeybinds::default());
        assert_eq!(press(&keymap, "1"), Some(Binding::Tool(Some(0))));
        assert_eq!(press(&keymap, "2"), Some(Binding::Tool(Some(1))));
        assert_eq!(press(&keymap, "3"), Some(Binding::Tool(Some(2))));
        assert_eq!(press(&keymap, "4"), None, "no fourth tool");
        assert_eq!(keymap.tool_labels(), ["1", "2", "3"]);
        assert_eq!(keymap.select_label(), "0");
    }

    #[test]
    fn unbound_keys_do_nothing() {
        let keymap = keymap(ConfigKeybinds::default());
        assert_eq!(keymap.lookup(Mods::default(), code("q")), None);
        assert_eq!(keymap.lookup(ctrl(), code("q")), None);
        // A modifier the bind doesn't ask for is not the same bind.
        assert_eq!(keymap.lookup(ctrl(), code("1")), None);
        assert_eq!(press(&keymap, "Ctrl+Shift+Z"), None);
    }

    #[test]
    fn the_config_remaps_tools_and_actions() {
        let mut config = ConfigKeybinds { undo: "Backspace".into(), ..Default::default() };
        config.tools.insert("line".to_string(), Keys::many(["l", "Ctrl+L"]));

        let keymap = keymap(config);
        assert_eq!(press(&keymap, "Backspace"), Some(Binding::Undo));
        assert_eq!(press(&keymap, "Ctrl+Z"), None, "the default is replaced, not kept");
        assert_eq!(press(&keymap, "l"), Some(Binding::Tool(Some(2))));
        assert_eq!(press(&keymap, "Ctrl+L"), Some(Binding::Tool(Some(2))));
        assert_eq!(press(&keymap, "3"), None, "'line' no longer sits on its digit");
        assert_eq!(press(&keymap, "1"), Some(Binding::Tool(Some(0))), "others keep theirs");
        assert_eq!(keymap.tool_labels(), ["1", "2", "L"], "the first bind, as written");
    }

    #[test]
    fn an_empty_list_unbinds_an_action() {
        let keymap = keymap(ConfigKeybinds { copy: Keys::many([]), ..Default::default() });
        assert_eq!(press(&keymap, "Return"), None);
        assert_eq!(press(&keymap, "Escape"), Some(Binding::Cancel), "others are untouched");
    }

    #[test]
    fn a_key_bound_twice_keeps_its_first_binding() {
        let mut config = ConfigKeybinds { save: "Escape".into(), ..Default::default() };
        config.tools.insert("brush".to_string(), "Escape".into());
        let keymap = keymap(config);
        assert_eq!(press(&keymap, "Escape"), Some(Binding::Cancel));
        assert!(keymap.tool_labels()[0].is_empty(), "the brush ended up unbound");
    }

    #[test]
    fn a_tool_taking_another_tools_digit_wins_over_the_default() {
        let mut config = ConfigKeybinds::default();
        config.tools.insert("line".to_string(), "1".into());
        let keymap = keymap(config);
        // 'line' is configured, the brush only defaults to that digit.
        assert_eq!(press(&keymap, "1"), Some(Binding::Tool(Some(2))));
        assert_eq!(keymap.tool_labels(), ["", "2", "1"]);
    }

    #[test]
    fn binds_for_unknown_tools_and_broken_specs_are_skipped() {
        let mut config = ConfigKeybinds::default();
        config.tools.insert("blur".to_string(), "5".into());
        config.tools.insert("arrow".to_string(), "Ctrl+Nope".into());
        let keymap = keymap(config);
        assert_eq!(press(&keymap, "5"), None);
        assert!(keymap.tool_labels()[1].is_empty());
        assert_eq!(press(&keymap, "1"), Some(Binding::Tool(Some(0))), "the rest still work");
    }

    #[test]
    fn tools_past_the_ninth_start_out_unbound() {
        let ids = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
        let keymap = Keymap::new(&ConfigKeybinds::default(), &ids);
        assert_eq!(keymap.lookup(Mods::default(), code("9")), Some(Binding::Tool(Some(8))));
        assert!(keymap.tool_labels()[9].is_empty());
    }
}
