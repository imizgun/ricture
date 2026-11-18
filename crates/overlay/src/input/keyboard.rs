use crate::state::{App, Binding, ExitFlag};
use ricture_config::keybinds::Mods;
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers,
};
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::protocol::{wl_keyboard, wl_surface};
use wayland_client::{Connection, QueueHandle};

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        if self.layer.wl_surface() == surface {
            self.keyboard_focus = true;
        }
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        if self.layer.wl_surface() == surface {
            self.keyboard_focus = false;
        }
    }

    /// Every key goes through the keymap; one that isn't bound does nothing,
    /// so the overlay stays open.
    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if !self.keyboard_focus {
            return;
        }

        // Binds are keyed by position (`raw_code`), not by the symbol the
        // layout produces, so they also work on a non-latin layout.
        let Some(binding) = self.keymap.lookup(self.mods, event.raw_code) else {
            return;
        };

        match binding {
            Binding::Cancel => self.exit_flag = Some(ExitFlag::Cancelled),
            Binding::Copy => self.exit_flag = Some(ExitFlag::Copy),
            Binding::Save => self.exit_flag = Some(ExitFlag::Save),
            // While a gesture is in progress the pointer owns the document;
            // both of these wait for the release.
            Binding::Undo if !self.document.is_drawing() => {
                // The renderer notices the revision bump and redraws.
                self.document.undo();
            }
            Binding::Tool(index) if !self.document.is_drawing() => self.toolbox.select(index),
            Binding::Undo | Binding::Tool(_) => {}
        }
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        // Caps/num lock are deliberately left out: they are states, not keys
        // a bind is held with.
        self.mods = Mods {
            ctrl: modifiers.ctrl,
            shift: modifiers.shift,
            alt: modifiers.alt,
            logo: modifiers.logo,
        };
    }
}
