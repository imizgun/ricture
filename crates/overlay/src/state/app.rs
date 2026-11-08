use crate::input::selection::Selection;
use crate::render::renderer::Renderer;
use crate::state::config::AppConfig;
use crate::state::exit_flag::ExitFlag;
use crate::state::toolbox::Toolbox;
use crate::ui::{Ui, UiEvent};
use ricture_tools::{Document, Style};
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

    pub(crate) toolbox: Toolbox,
    pub(crate) document: Document,
    /// The in-progress gesture changed and needs a redraw.
    pub(crate) canvas_dirty: bool,

    pub(crate) config: AppConfig,
}

impl App {
    /// Maps a surface-local (logical) pointer position to buffer/screenshot
    /// pixels, clamped to the image bounds (implicit grabs can report
    /// positions outside the surface while dragging).
    pub(crate) fn to_buffer_coords(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let (sx, sy) = self.scale();
        let bw = self.screenshot.width as f64;
        let bh = self.screenshot.height as f64;
        ((x * sx).clamp(0.0, bw), (y * sy).clamp(0.0, bh))
    }

    /// Buffer pixels per logical pixel, per axis.
    pub(crate) fn scale(&self) -> (f64, f64) {
        let ratio = |buffer: u32, logical: u32| {
            if logical > 0 { buffer as f64 / logical as f64 } else { 1.0 }
        };
        (
            ratio(self.screenshot.width, self.width),
            ratio(self.screenshot.height, self.height),
        )
    }

    /// The finished selection, if any, in buffer pixels. Tiny rects (a stray
    /// click) don't count.
    pub(crate) fn settled_selection(&self) -> Option<tiny_skia::Rect> {
        if self.selection.is_dragging {
            return None;
        }
        self.selection
            .rect()
            .filter(|r| r.width() >= 2.0 && r.height() >= 2.0)
    }

    /// Style for a new gesture from the current settings.
    pub(crate) fn current_style(&self) -> Style {
        self.toolbox.style(self.scale().0 as f32)
    }

    /// Applies what the user asked for through the UI since the last call.
    pub(crate) fn apply_ui_events(&mut self) {
        let Some(ui) = &self.ui else { return };
        for event in ui.take_events() {
            match event {
                UiEvent::ToolSelected(index) => self.toolbox.select(index),
                UiEvent::ColorPicked(index) => self.toolbox.pick_color(index),
                UiEvent::WidthChanged(value) => self.toolbox.set_width(value),
                UiEvent::WidthScrolled(steps) => self.toolbox.scroll_width(steps),
                UiEvent::PickerToggled => self.toolbox.toggle_picker(),
                UiEvent::PickerHueChanged(hue) => self.toolbox.set_picked_hue(hue),
                UiEvent::PickerShadeChanged(saturation, value) => {
                    self.toolbox.set_picked_shade(saturation, value)
                }
            }
        }
    }

    /// Pushes the current state into the UI, which only displays it.
    pub(crate) fn sync_ui(&self) {
        let Some(ui) = &self.ui else { return };
        let (sx, sy) = self.scale();
        let area = self.settled_selection().and_then(|r| {
            tiny_skia::Rect::from_xywh(
                r.x() / sx as f32,
                r.y() / sy as f32,
                r.width() / sx as f32,
                r.height() / sy as f32,
            )
        });
        ui.set_selection(area);
        ui.set_tool_state(&self.toolbox);
    }
}
