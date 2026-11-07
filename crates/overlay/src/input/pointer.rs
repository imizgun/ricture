use crate::state::App;
use smithay_client_toolkit::seat::pointer::{
    CursorIcon, PointerEvent, PointerEventKind, PointerHandler,
};
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::protocol::wl_pointer;
use wayland_client::{Connection, QueueHandle};

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }

            // Slint works in logical (surface) coordinates, the selection in
            // buffer pixels.
            let logical = event.position;
            let position = self.to_buffer_coords(logical);

            match event.kind {
                Enter { .. } | Motion { .. } => {
                    if let Some(ui) = &self.ui {
                        ui.pointer_moved(logical);
                    }
                    // The compositor forgets our cursor between leave/enter.
                    let force = matches!(event.kind, Enter { .. });
                    self.update_cursor(conn, logical, force);

                    if self.selection.is_dragging {
                        self.selection.current = Some(position);
                    }
                }
                Leave { .. } => {
                    if let Some(ui) = &self.ui {
                        ui.pointer_exited();
                    }
                }
                Axis { .. } => {}
                Press { button, .. } => {
                    if button != BTN_LEFT {
                        continue;
                    }
                    if let Some(ui) = self.ui.as_ref().filter(|ui| ui.hit(logical)) {
                        ui.pointer_pressed(logical);
                        self.ui_grab = true;
                        continue;
                    }
                    self.selection.start = Some(position);
                    self.selection.current = Some(position);
                    self.selection.is_dragging = true;
                }
                Release { button, .. } => {
                    if button != BTN_LEFT {
                        continue;
                    }
                    if self.ui_grab {
                        self.ui_grab = false;
                        if let Some(ui) = &self.ui {
                            ui.pointer_released(logical);
                        }
                        continue;
                    }
                    if self.selection.is_dragging {
                        self.selection.current = Some(position);
                        self.selection.is_dragging = false;
                    }
                }
            }
        }
    }
}

/// `BTN_LEFT` from linux/input-event-codes.h.
const BTN_LEFT: u32 = 0x110;

impl App {
    /// Default arrow over the UI, crosshair over the screenshot.
    fn update_cursor(&mut self, conn: &Connection, logical: (f64, f64), force: bool) {
        let over_ui = !self.selection.is_dragging
            && (self.ui_grab || self.ui.as_ref().is_some_and(|ui| ui.hit(logical)));
        if over_ui == self.cursor_over_ui && !force {
            return;
        }
        self.cursor_over_ui = over_ui;
        if let Some(pointer) = &self.pointer {
            let icon = if over_ui { CursorIcon::Default } else { CursorIcon::Crosshair };
            let _ = pointer.set_cursor(conn, icon);
        }
    }
}
