use crate::input::selection::Selection;
use crate::render::renderer::Renderer;
use crate::state::config::AppConfig;
use crate::state::exit_flag::ExitFlag;
use crate::ui::Ui;
use ricture_capture::Screenshot;
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::seat::pointer::ThemedPointer;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::shm::Shm;
use wayland_client::protocol::wl_keyboard;

pub(crate) struct App {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) compositor: CompositorState,
    pub(crate) shm: Shm,

    pub(crate) exit_flag: Option<ExitFlag>,
    pub(crate) first_configure: bool,
    /// Logical surface size from the layer-shell configure. Pointer events
    /// arrive in this coordinate space.
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Scales the physical-size buffer (= screenshot size) down to the
    /// logical surface size. `None` if the compositor lacks wp_viewporter.
    pub(crate) viewport: Option<WpViewport>,

    pub(crate) layer: LayerSurface,
    pub(crate) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(crate) keyboard_focus: bool,
    pub(crate) ctrl_held: bool,
    pub(crate) pointer: Option<ThemedPointer>,

    pub(crate) screenshot: Screenshot,
    pub(crate) renderer: Renderer,
    pub(crate) selection: Selection,
    /// Created on first configure, once the surface scale is known.
    pub(crate) ui: Option<Ui>,
    /// A press started on the UI; the matching release belongs to it too.
    pub(crate) ui_grab: bool,
    pub(crate) cursor_over_ui: bool,

    pub(crate) config: AppConfig,
}

impl App {
    /// Maps a surface-local (logical) pointer position to buffer/screenshot
    /// pixels, clamped to the image bounds (implicit grabs can report
    /// positions outside the surface while dragging).
    pub(crate) fn to_buffer_coords(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let bw = self.screenshot.width as f64;
        let bh = self.screenshot.height as f64;
        let sx = if self.width > 0 { bw / self.width as f64 } else { 1.0 };
        let sy = if self.height > 0 { bh / self.height as f64 } else { 1.0 };
        ((x * sx).clamp(0.0, bw), (y * sy).clamp(0.0, bh))
    }
}
