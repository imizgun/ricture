use crate::state::{App, ExitFlag};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell};
use smithay_client_toolkit::shm::Shm;
use wayland_client::Connection;
use wayland_client::globals::registry_queue_init;

/// On confirm: `(x, y, width, height)` of the selection (already normalized —
/// drag direction doesn't matter) plus the `Screenshot` handed back, since
/// `App` had to take ownership of it and this is the only way to get it back.
/// On cancel: `None`.
pub fn run(
    screenshot: ricture_capture::Screenshot,
) -> Result<Option<((f64, f64, f64, f64), ricture_capture::Screenshot)>, Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;

    let surface = compositor.create_surface(&qh);

    let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("ricture-overlay"), None);

    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);

    layer.commit();

    let mut app = App {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,

        exit_flag: None,
        first_configure: true,
        pool: None,
        width: 0,
        height: 0,

        layer,
        keyboard: None,
        keyboard_focus: false,
        pointer: None,

        screenshot,
        pixmap: None,
        selection_start: None,
        selection_current: None,
        prev_selection_start: None,
        prev_selection_current: None,
        is_dragging: false
    };

    while !app.exit_flag.is_some() {
        event_queue.blocking_dispatch(&mut app)?;
    }

    match app.exit_flag {
        Some(ExitFlag::FrameConfirmed) => {
            let (x0, y0) = app.selection_start.unwrap();
            let (x1, y1) = app.selection_current.unwrap();
            let rect = (x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs());
            Ok(Some((rect, app.screenshot)))
        }
        Some(ExitFlag::Cancelled) | None => Ok(None),
    }
}
