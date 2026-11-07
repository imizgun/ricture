mod platform;

slint::include_modules!();

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{PointerEventButton, WindowEvent};
use ricture_tools::{Setting, ToolMeta};
use slint::{ComponentHandle, LogicalPosition, ModelRc, PhysicalSize, PlatformError, VecModel};
use std::rc::Rc;
use tiny_skia::{Color, Pixmap, Rect};

pub(crate) struct Ui {
    window: Rc<MinimalSoftwareWindow>,
    component: OverlayUi,
    
    /// Last rendered UI frame, premultiplied RGBA at buffer resolution.
    pub(crate) pixmap: Pixmap,
}

impl Ui {
    /// `width`/`height` are the buffer size in physical pixels; `scale` maps
    /// logical surface coordinates to them.
    pub(crate) fn new(
        width: u32,
        height: u32,
        scale: f32,
        tools: impl IntoIterator<Item = ToolMeta>,
    ) -> Result<Self, PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        platform::install(window.clone())?;

        let component = OverlayUi::new()?;
        let tools: Vec<ToolInfo> = tools.into_iter().map(tool_info).collect();
        component.set_tools(ModelRc::new(VecModel::from(tools)));
        window.dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: scale,
        });
        window.set_size(PhysicalSize::new(width, height));
        component.show()?;

        let pixmap = Pixmap::new(width, height).expect("non-zero surface size");
        Ok(Self {
            window,
            component,
            pixmap,
        })
    }

    /// Index into the registered tools, `None` in Select mode.
    pub(crate) fn active_tool(&self) -> Option<usize> {
        usize::try_from(self.component.get_active_tool()).ok()
    }

    pub(crate) fn stroke_color(&self) -> Color {
        let c = self.component.get_stroke_color();
        Color::from_rgba8(c.red(), c.green(), c.blue(), c.alpha())
    }

    /// Stroke width in logical px.
    pub(crate) fn stroke_width(&self) -> f32 {
        self.component.get_stroke_width()
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
        self.window.draw_if_needed(|renderer| {
            let pixels: &mut [PremultipliedRgbaColor] = bytemuck::cast_slice_mut(pixmap.data_mut());
            renderer.render(pixels, width);
        })
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
}

fn logical((x, y): (f64, f64)) -> LogicalPosition {
    LogicalPosition::new(x as f32, y as f32)
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
