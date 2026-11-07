use crate::input::selection::Selection;
use crate::render::renderer::Renderer;
use crate::state::{Action, App, AppConfig, ExitFlag};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell};
use crate::handlers::viewporter::ViewportData;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::shm::Shm;
use wayland_client::Connection;
use wayland_client::globals::registry_queue_init;

pub fn run(
    screenshot: ricture_capture::Screenshot,
    config: AppConfig,
) -> Result<
    Option<(Action, (f64, f64, f64, f64), ricture_capture::Screenshot)>,
    Box<dyn std::error::Error>,
> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;

    let surface = compositor.create_surface(&qh);

    // The screenshot is in physical pixels while the layer surface is sized
    // in logical ones; with fractional scaling (common on 16:10 laptop
    // panels) the two differ, so render at full resolution and let the
    // viewport scale it down to the surface size.
    let viewport = globals
        .bind::<WpViewporter, _, _>(&qh, 1..=1, ViewportData)
        .ok()
        .map(|viewporter| viewporter.get_viewport(&surface, &qh, ViewportData));

    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Overlay,
        Some("ricture-overlay"),
        None,
    );

    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);

    layer.commit();

    let mut app = App {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor,
        shm,

        exit_flag: None,
        first_configure: true,
        width: 0,
        height: 0,
        viewport,

        layer,
        keyboard: None,
        keyboard_focus: false,
        ctrl_held: false,
        pointer: None,

        screenshot,
        renderer: Renderer::default(),
        selection: Selection::default(),
        ui: None,
        ui_grab: false,
        cursor_over_ui: false,

        config,
    };

    while !app.exit_flag.is_some() {
        event_queue.blocking_dispatch(&mut app)?;
    }

    match app.exit_flag {
        Some(ExitFlag::Copy) | Some(ExitFlag::Save) => {
            let action = match app.exit_flag {
                Some(ExitFlag::Copy) => Action::Copy,
                Some(ExitFlag::Save) => Action::Save,
                _ => unreachable!(),
            };
            let rect = app
                .selection
                .normalized()
                .expect("confirmed selection must have both endpoints");
            Ok(Some((action, rect, app.screenshot)))
        }
        Some(ExitFlag::Cancelled) | None => Ok(None),
    }
}
