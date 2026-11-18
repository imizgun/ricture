mod platform;

slint::include_modules!();

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{PointerEventButton, WindowAdapter, WindowEvent};
use crate::render::blit::VisibleRuns;
use crate::state::{Keymap, Toolbox};
use ricture_tools::{Setting, ToolMeta};
use slint::{ComponentHandle, LogicalPosition, ModelRc, PhysicalSize, PlatformError, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use tiny_skia::{Color, Pixmap, Rect};

/// Something the user asked for through the UI. Slint callbacks can't reach
/// `App`, so they queue these for it to apply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum UiEvent {
    /// Index into the registered tools, `None` for Select.
    ToolSelected(Option<usize>),
    /// Index into the palette.
    ColorPicked(usize),
    /// Requested stroke width in logical px, not yet validated.
    WidthChanged(f32),
    /// Wheel notches over the width slider, positive means up.
    WidthScrolled(f32),
    PickerToggled,
    /// Requested hue, not yet validated.
    PickerHueChanged(f32),
    /// Requested saturation and value, not yet validated.
    PickerShadeChanged(f32, f32),
}

type EventQueue = Rc<RefCell<Vec<UiEvent>>>;

pub(crate) struct Ui {
    window: Rc<MinimalSoftwareWindow>,
    component: OverlayUi,
    events: EventQueue,

    /// Buffer pixels per logical pixel.
    scale: f32,
    /// Screen (= buffer) size in physical pixels.
    screen: (u32, u32),
    /// The window only covers the panels; this is where it sits on screen,
    /// in physical pixels. Zero-sized while nothing is shown.
    bounds: IntRect,

    /// Last rendered UI frame, premultiplied RGBA at buffer resolution,
    /// `bounds`-sized. `None` while nothing is shown.
    pixmap: Option<Pixmap>,
    /// The next render must repaint the whole window: its buffer is new or
    /// was not kept up to date.
    full_repaint: bool,
    visible: VisibleRuns,
}

/// Rectangle in physical pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct IntRect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Ui {
    /// `width`/`height` are the buffer size in physical pixels; `scale` maps
    /// logical surface coordinates to them. The toolbar and the keybinds it
    /// advertises don't change while the overlay is open, so they are only
    /// read here.
    pub(crate) fn new(
        width: u32,
        height: u32,
        scale: f32,
        toolbox: &Toolbox,
        keymap: &Keymap,
    ) -> Result<Self, PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        platform::install(window.clone())?;

        let component = OverlayUi::new()?;
        let shortcuts = keymap.tool_labels();
        let tools: Vec<ToolInfo> = toolbox
            .metas()
            .enumerate()
            .map(|(index, meta)| {
                tool_info(meta, shortcuts.get(index).map_or("", String::as_str))
            })
            .collect();
        component.set_tools(ModelRc::new(VecModel::from(tools)));
        component.set_select_shortcut(keymap.select_label().into());
        let swatches: Vec<slint::Color> =
            toolbox.palette().iter().copied().map(to_slint_color).collect();
        component.set_swatches(ModelRc::new(VecModel::from(swatches)));

        let events = EventQueue::default();
        let send = sender(&events);
        component.on_tool_selected(move |index| {
            send(UiEvent::ToolSelected(usize::try_from(index).ok()));
        });
        let send = sender(&events);
        component.on_color_picked(move |index| {
            if let Ok(index) = usize::try_from(index) {
                send(UiEvent::ColorPicked(index));
            }
        });
        let send = sender(&events);
        component.on_width_changed(move |value| send(UiEvent::WidthChanged(value)));
        let send = sender(&events);
        component.on_width_scrolled(move |steps| send(UiEvent::WidthScrolled(steps)));
        let send = sender(&events);
        component.on_picker_toggled(move || send(UiEvent::PickerToggled));
        let send = sender(&events);
        component.on_picker_hue_changed(move |hue| send(UiEvent::PickerHueChanged(hue)));
        let send = sender(&events);
        component.on_picker_shade_changed(move |saturation, value| {
            send(UiEvent::PickerShadeChanged(saturation, value));
        });

        window.dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: scale,
        });
        component.set_screen_width(width as f32 / scale);
        component.set_screen_height(height as f32 / scale);
        // Resized to the panels once they are shown.
        window.set_size(PhysicalSize::new(1, 1));
        component.show()?;

        Ok(Self {
            window,
            component,
            events,
            scale,
            screen: (width, height),
            bounds: IntRect::default(),
            pixmap: None,
            full_repaint: true,
            visible: VisibleRuns::default(),
        })
    }

    /// Drains what the user asked for since the last call, oldest first.
    pub(crate) fn take_events(&self) -> Vec<UiEvent> {
        self.events.take()
    }

    /// Displays the current tool settings.
    pub(crate) fn set_tool_state(&self, toolbox: &Toolbox) {
        let ui = &self.component;
        ui.set_active_tool(toolbox.active_index().map_or(-1, |index| index as i32));
        ui.set_stroke_color(to_slint_color(toolbox.color()));
        ui.set_stroke_width(toolbox.width());
        ui.set_color_is_custom(toolbox.is_custom_color());

        let picked = toolbox.picked();
        ui.set_picker_open(toolbox.is_picker_open());
        ui.set_picker_hue(picked.hue);
        ui.set_picker_saturation(picked.saturation);
        ui.set_picker_value(picked.value);
        ui.set_picker_hue_color(to_slint_color(toolbox.picked_hue_color()));
    }

    /// Shows the toolbar around `area` (logical px), or hides it.
    pub(crate) fn set_selection(&self, area: Option<Rect>) {
        self.component.set_panels_visible(area.is_some());
        if let Some(area) = area {
            self.component.set_sel_x(area.x());
            self.component.set_sel_y(area.y());
            self.component.set_sel_w(area.width());
            self.component.set_sel_h(area.height());
        }
    }

    /// Advances Slint timers/animations, fits the window to the panels and
    /// re-renders dirty regions. Returns whether what `composite_onto` draws
    /// changed.
    pub(crate) fn render(&mut self) -> bool {
        slint::platform::update_timers_and_animations();
        let layout_changed = self.fit_to_content();

        let Some(pixmap) = self.pixmap.as_mut() else {
            return layout_changed;
        };
        let full_repaint = std::mem::take(&mut self.full_repaint);
        if full_repaint {
            self.window.request_redraw();
        }
        let width = pixmap.width() as usize;
        let span = ricture_profile::span("frame: ui render");
        let rendered = self.window.draw_if_needed(|renderer| {
            // Partial rendering assumes the buffer still holds the last frame.
            if full_repaint {
                renderer.set_repaint_buffer_type(RepaintBufferType::NewBuffer);
            }
            let pixels: &mut [PremultipliedRgbaColor] = bytemuck::cast_slice_mut(pixmap.data_mut());
            renderer.render(pixels, width);
            if full_repaint {
                renderer.set_repaint_buffer_type(RepaintBufferType::ReusedBuffer);
            }
        });
        if rendered {
            self.visible.rebuild(pixmap);
        } else {
            span.cancel();
        }
        rendered || layout_changed
    }

    /// Sizes and places the window to cover what the UI shows. Returns
    /// whether that changed.
    fn fit_to_content(&mut self) -> bool {
        let ui = &self.component;
        let bounds = to_physical(
            (ui.get_content_x(), ui.get_content_y()),
            (ui.get_content_width(), ui.get_content_height()),
            self.scale,
            self.screen,
        );
        if bounds == self.bounds {
            return false;
        }
        let resized = (bounds.width, bounds.height) != (self.bounds.width, self.bounds.height);
        self.bounds = bounds;

        // Snapped to whole buffer pixels, so the window's pixels line up with
        // the screen's.
        ui.set_origin_x(bounds.x as f32 / self.scale);
        ui.set_origin_y(bounds.y as f32 / self.scale);

        if bounds.width == 0 || bounds.height == 0 {
            self.pixmap = None;
            // Slint keeps tracking changes while we don't render; start over.
            self.full_repaint = true;
        } else if resized || self.pixmap.is_none() {
            self.window.set_size(PhysicalSize::new(bounds.width, bounds.height));
            self.pixmap = Pixmap::new(bounds.width, bounds.height);
            self.full_repaint = true;
        }
        true
    }

    /// Blends the last rendered UI frame over `target`.
    pub(crate) fn composite_onto(&self, target: &mut Pixmap) {
        if let Some(pixmap) = &self.pixmap {
            self.visible.composite(pixmap, target, (self.bounds.x, self.bounds.y));
        }
    }

    /// Screen position (logical px) in window coordinates.
    fn to_window(&self, (x, y): (f64, f64)) -> LogicalPosition {
        let scale = self.scale as f64;
        LogicalPosition::new(
            (x - self.bounds.x as f64 / scale) as f32,
            (y - self.bounds.y as f64 / scale) as f32,
        )
    }

    pub(crate) fn hit(&self, (x, y): (f64, f64)) -> bool {
        self.component.invoke_hit(x as f32, y as f32)
    }

    pub(crate) fn pointer_moved(&self, pos: (f64, f64)) {
        self.window.dispatch_event(WindowEvent::PointerMoved {
            position: self.to_window(pos),
        });
    }

    pub(crate) fn pointer_pressed(&self, pos: (f64, f64)) {
        self.window.dispatch_event(WindowEvent::PointerPressed {
            position: self.to_window(pos),
            button: PointerEventButton::Left,
        });
    }

    pub(crate) fn pointer_released(&self, pos: (f64, f64)) {
        self.window.dispatch_event(WindowEvent::PointerReleased {
            position: self.to_window(pos),
            button: PointerEventButton::Left,
        });
    }

    pub(crate) fn pointer_exited(&self) {
        self.window.dispatch_event(WindowEvent::PointerExited);
    }

    /// `steps` are wheel notches; the UI receives them as a length, one
    /// logical px per notch.
    pub(crate) fn pointer_scrolled(&self, pos: (f64, f64), steps: f32) {
        self.window.dispatch_event(WindowEvent::PointerScrolled {
            position: self.to_window(pos),
            delta_x: 0.0,
            delta_y: steps,
        });
    }
}

fn sender(events: &EventQueue) -> impl Fn(UiEvent) + 'static {
    let events = events.clone();
    move |event| events.borrow_mut().push(event)
}

/// Logical rect to whole physical pixels covering it, clipped to the screen.
fn to_physical(
    (x, y): (f32, f32),
    (width, height): (f32, f32),
    scale: f32,
    (screen_width, screen_height): (u32, u32),
) -> IntRect {
    if width <= 0.0 || height <= 0.0 {
        return IntRect::default();
    }
    let clamp = |value: f32, max: u32| value.clamp(0.0, max as f32) as u32;
    let left = clamp((x * scale).floor(), screen_width);
    let top = clamp((y * scale).floor(), screen_height);
    let right = clamp(((x + width) * scale).ceil(), screen_width);
    let bottom = clamp(((y + height) * scale).ceil(), screen_height);
    IntRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

fn to_slint_color(color: Color) -> slint::Color {
    let c = color.to_color_u8();
    slint::Color::from_argb_u8(c.alpha(), c.red(), c.green(), c.blue())
}

fn tool_info(meta: ToolMeta, shortcut: &str) -> ToolInfo {
    let mut info = ToolInfo {
        name: meta.name.into(),
        icon: meta.icon.into(),
        shortcut: shortcut.into(),
        ..Default::default()
    };
    for setting in meta.settings {
        match *setting {
            Setting::Color => info.has_color = true,
            Setting::Width { min, max } => {
                info.has_width = true;
                info.width_min = min;
                info.width_max = max;
            }
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_bounds_cover_fractional_edges() {
        let rect = to_physical((10.3, 20.0), (100.0, 50.5), 1.75, (2880, 1800));
        assert_eq!(rect, IntRect { x: 18, y: 35, width: 176, height: 89 });
    }

    #[test]
    fn physical_bounds_are_clipped_to_the_screen() {
        let rect = to_physical((-40.0, 1000.0), (100.0, 50.0), 2.0, (1000, 2050));
        assert_eq!(rect, IntRect { x: 0, y: 2000, width: 120, height: 50 });
    }

    #[test]
    fn empty_content_has_no_bounds() {
        assert_eq!(to_physical((5.0, 5.0), (0.0, 30.0), 1.0, (100, 100)), IntRect::default());
    }
}
