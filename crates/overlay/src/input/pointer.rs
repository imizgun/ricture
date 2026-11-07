use crate::state::App;
use smithay_client_toolkit::seat::pointer::{
    CursorIcon, PointerEvent, PointerEventKind, PointerHandler,
};
use smithay_client_toolkit::shell::WaylandSurface;
use tiny_skia::Point;
use wayland_client::protocol::wl_pointer;
use wayland_client::{Connection, QueueHandle};

/// `BTN_LEFT` from linux/input-event-codes.h.
const BTN_LEFT: u32 = 0x110;

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

            // Slint works in logical (surface) coordinates; selection and
            // annotations in buffer pixels.
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

                    if self.document.is_drawing() {
                        self.document.update(point(position));
                        self.canvas_dirty = true;
                    } else if self.selection.is_dragging {
                        self.selection.current = Some(position);
                    }
                }
                Leave { .. } => {
                    if let Some(ui) = &self.ui {
                        ui.pointer_exited();
                    }
                }
                Axis { .. } => {}
                Press { button, .. } if button == BTN_LEFT => self.press(logical, position),
                Release { button, .. } if button == BTN_LEFT => self.release(logical, position),
                Press { .. } | Release { .. } => {}
            }
        }
    }
}

impl App {
    fn press(&mut self, logical: (f64, f64), position: (f64, f64)) {
        if let Some(ui) = self.ui.as_ref().filter(|ui| ui.hit(logical)) {
            ui.pointer_pressed(logical);
            self.ui_grab = true;
            return;
        }

        let active_tool = self.ui.as_ref().and_then(|ui| ui.active_tool());
        match active_tool.and_then(|index| self.tools.get(index)) {
            // A drawing tool: draw, but only starting inside the area.
            Some(tool) => {
                let inside = self
                    .settled_selection()
                    .is_some_and(|r| {
                        let (x, y) = (position.0 as f32, position.1 as f32);
                        x >= r.left() && x <= r.right() && y >= r.top() && y <= r.bottom()
                    });
                if let (true, Some(style)) = (inside, self.current_style()) {
                    self.document.begin(tool.as_ref(), point(position), style);
                    self.canvas_dirty = true;
                }
            }
            // Select mode: start a new selection.
            None => {
                self.selection.start = Some(position);
                self.selection.current = Some(position);
                self.selection.is_dragging = true;
            }
        }
    }

    fn release(&mut self, logical: (f64, f64), position: (f64, f64)) {
        if self.ui_grab {
            self.ui_grab = false;
            if let Some(ui) = &self.ui {
                ui.pointer_released(logical);
            }
        } else if self.document.is_drawing() {
            self.document.update(point(position));
            self.document.finish();
            self.canvas_dirty = true;
        } else if self.selection.is_dragging {
            self.selection.current = Some(position);
            self.selection.is_dragging = false;
        }
    }

    /// Default arrow over the UI, crosshair elsewhere.
    fn update_cursor(&mut self, conn: &Connection, logical: (f64, f64), force: bool) {
        let busy = self.selection.is_dragging || self.document.is_drawing();
        let over_ui =
            !busy && (self.ui_grab || self.ui.as_ref().is_some_and(|ui| ui.hit(logical)));
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

fn point((x, y): (f64, f64)) -> Point {
    Point::from_xy(x as f32, y as f32)
}
