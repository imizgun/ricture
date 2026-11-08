mod platform;

slint::include_modules!();

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{PointerEventButton, WindowEvent};
use crate::render::blit::VisibleRuns;
use crate::state::Toolbox;
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


    /// Last rendered UI frame, premultiplied RGBA at buffer resolution.
    pub(crate) pixmap: Pixmap,
    visible: VisibleRuns,
}

impl Ui {
    /// `width`/`height` are the buffer size in physical pixels; `scale` maps
    /// logical surface coordinates to them.
    pub(crate) fn new(
        width: u32,
        height: u32,
        scale: f32,
        tools: impl IntoIterator<Item = ToolMeta>,
        palette: &[Color],
    ) -> Result<Self, PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        platform::install(window.clone())?;

        let component = OverlayUi::new()?;
        let tools: Vec<ToolInfo> = tools.into_iter().map(tool_info).collect();
        component.set_tools(ModelRc::new(VecModel::from(tools)));
        let swatches: Vec<slint::Color> = palette.iter().copied().map(to_slint_color).collect();
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
        window.set_size(PhysicalSize::new(width, height));
        component.show()?;

        let pixmap = Pixmap::new(width, height).expect("non-zero surface size");
        Ok(Self {
            window,
            component,
            events,
            pixmap,
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

    /// Advances Slint timers/animations and re-renders dirty regions.
    /// Returns whether the UI pixmap changed.
    pub(crate) fn render(&mut self) -> bool {
        slint::platform::update_timers_and_animations();
        let width = self.pixmap.width() as usize;
        let pixmap = &mut self.pixmap;
        let changed = self.window.draw_if_needed(|renderer| {
            let pixels: &mut [PremultipliedRgbaColor] = bytemuck::cast_slice_mut(pixmap.data_mut());
            renderer.render(pixels, width);
        });
        if changed {
            self.visible.rebuild(&self.pixmap);
        }
        changed
    }

    /// Blends the last rendered UI frame over `target`.
    pub(crate) fn composite_onto(&self, target: &mut Pixmap) {
        self.visible.composite(&self.pixmap, target);
    }

    pub(crate) fn hit(&self, (x, y): (f64, f64)) -> bool {
        self.component.invoke_hit(x as f32, y as f32)
    }

    pub(crate) fn pointer_moved(&self, pos: (f64, f64)) {
        self.window.dispatch_event(WindowEvent::PointerMoved {
            position: logical(pos),
        });
    }

    pub(crate) fn pointer_pressed(&self, pos: (f64, f64)) {
        self.window.dispatch_event(WindowEvent::PointerPressed {
            position: logical(pos),
            button: PointerEventButton::Left,
        });
    }

    pub(crate) fn pointer_released(&self, pos: (f64, f64)) {
        self.window.dispatch_event(WindowEvent::PointerReleased {
            position: logical(pos),
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
            position: logical(pos),
            delta_x: 0.0,
            delta_y: steps,
        });
    }
}

fn sender(events: &EventQueue) -> impl Fn(UiEvent) + 'static {
    let events = events.clone();
    move |event| events.borrow_mut().push(event)
}

fn logical((x, y): (f64, f64)) -> LogicalPosition {
    LogicalPosition::new(x as f32, y as f32)
}

fn to_slint_color(color: Color) -> slint::Color {
    let c = color.to_color_u8();
    slint::Color::from_argb_u8(c.alpha(), c.red(), c.green(), c.blue())
}

fn tool_info(meta: ToolMeta) -> ToolInfo {
    let mut info = ToolInfo {
        name: meta.name.into(),
        icon: meta.icon.into(),
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
