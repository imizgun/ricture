use crate::state::App;
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::protocol::wl_pointer;
use wayland_client::{Connection, QueueHandle};

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }

            match event.kind {
                Enter { .. } | Leave { .. } | Axis { .. } => {}

                Motion { .. } => {
                    if self.is_dragging {
                        self.selection_current = Some(event.position);
                    }
                }
                Press { .. } => {
                    self.selection_start = Some(event.position);
                    self.is_dragging = true;
                }
                Release { .. } => {
                    self.selection_current = Some(event.position);
                    self.is_dragging = false;
                }
            }
        }
    }
}
