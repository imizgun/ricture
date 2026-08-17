use crate::input::selection::Selection;
use crate::render::renderer::Renderer;
use crate::state::config::AppConfig;
use crate::state::exit_flag::ExitFlag;
use ricture_capture::Screenshot;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use smithay_client_toolkit::shm::Shm;
use wayland_client::protocol::{wl_keyboard, wl_pointer};

pub(crate) struct App {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) shm: Shm,

    pub(crate) exit_flag: Option<ExitFlag>,
    pub(crate) first_configure: bool,
    pub(crate) width: u32,
    pub(crate) height: u32,

    pub(crate) layer: LayerSurface,
    pub(crate) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(crate) keyboard_focus: bool,
    pub(crate) ctrl_held: bool,
    pub(crate) pointer: Option<wl_pointer::WlPointer>,

    pub(crate) screenshot: Screenshot,
    pub(crate) renderer: Renderer,
    pub(crate) selection: Selection,

    pub(crate) config: AppConfig,
}
