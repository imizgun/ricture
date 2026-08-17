use crate::renderer::Renderer;
use crate::selection::Selection;
use ricture_capture::Screenshot;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use smithay_client_toolkit::shm::Shm;
use tiny_skia::Color;
use wayland_client::protocol::{wl_keyboard, wl_pointer};
use ricture_config::config::Config;

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

#[derive(Copy, Clone)]
pub(crate) enum ExitFlag {
    Cancelled,
    Copy,
    Save,
}

pub enum Action {
    Copy,
    Save,
}

pub struct AppConfig {
    pub(crate) rect_color: Color,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig { rect_color: Color::from_rgba8(255, 255, 255, 255) }
    }
}

impl From<Config> for AppConfig {
    fn from(config: Config) -> Self {
        AppConfig { rect_color: parse_rrggbbaa(&config.appearance.rect_color) }
    }
}

fn parse_rrggbbaa(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).expect("hex color validated by ricture-config");
    let (rgb, a) = if hex.len() == 6 { (value, 0xff) } else { (value >> 8, value & 0xff) };
    Color::from_rgba8(((rgb >> 16) & 0xff) as u8, ((rgb >> 8) & 0xff) as u8, (rgb & 0xff) as u8, a as u8)
}