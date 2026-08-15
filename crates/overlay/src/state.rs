use ricture_capture::Screenshot;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::SlotPool;
use tiny_skia::Pixmap;
use wayland_client::protocol::{wl_keyboard, wl_pointer};

pub(crate) struct App {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) shm: Shm,

    pub(crate) exit_flag: Option<ExitFlag>,
    pub(crate) first_configure: bool,
    pub(crate) pool: Option<SlotPool>,
    pub(crate) width: u32,
    pub(crate) height: u32,

    pub(crate) layer: LayerSurface,
    pub(crate) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(crate) keyboard_focus: bool,
    pub(crate) pointer: Option<wl_pointer::WlPointer>,

    pub(crate) screenshot: Screenshot,
    pub(crate) pixmap: Option<Pixmap>,
    pub(crate) selection_start: Option<(f64, f64)>,
    pub(crate) selection_current: Option<(f64, f64)>,
    pub(crate) prev_selection_start: Option<(f64, f64)>,
    pub(crate) prev_selection_current: Option<(f64, f64)>,
    pub(crate) is_dragging: bool
}

#[derive(Copy, Clone)]
pub(crate) enum ExitFlag {
    Cancelled,
    FrameConfirmed,
}
