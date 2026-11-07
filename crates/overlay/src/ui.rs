mod platform;

slint::include_modules!();

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, PhysicalSize, PlatformError};
use std::rc::Rc;
use tiny_skia::Pixmap;

pub(crate) struct Ui {
    window: Rc<MinimalSoftwareWindow>,
    component: OverlayUi,
    
    /// Last rendered UI frame, premultiplied RGBA at buffer resolution.
    pub(crate) pixmap: Pixmap,
}

impl Ui {
    /// `width`/`height` are the buffer size in physical pixels; `scale` maps
    /// logical surface coordinates to them.
    pub(crate) fn new(width: u32, height: u32, scale: f32) -> Result<Self, PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        platform::install(window.clone())?;

        let component = OverlayUi::new()?;
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

    #[allow(dead_code)] // consumed once annotation tools land
    pub(crate) fn active_tool(&self) -> Tool {
        self.component.get_active_tool()
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
