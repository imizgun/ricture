use crate::state::{App, ExitFlag};
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::slot::SlotPool;
use std::num::NonZeroU32;
use wayland_client::{Connection, QueueHandle};

impl LayerShellHandler for App {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit_flag = Some(ExitFlag::Cancelled);
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.width = NonZeroU32::new(configure.new_size.0).map_or(1920, NonZeroU32::get);
        self.height = NonZeroU32::new(configure.new_size.1).map_or(1080, NonZeroU32::get);

        if let Some(viewport) = &self.viewport {
            viewport.set_destination(self.width as i32, self.height as i32);
        } else {
            // No viewporter: best effort with an integer buffer scale.
            let scale = (self.screenshot.width / self.width.max(1)).max(1);
            self.layer.wl_surface().set_buffer_scale(scale as i32);
        }

        if self.first_configure {
            self.first_configure = false;
            let byte_size =
                self.screenshot.width as usize * self.screenshot.height as usize * 4;
            self.renderer.pool =
                Some(SlotPool::new(byte_size, &self.shm).expect("failed to create shm pool"));
            self.draw(qh);
        }
    }
}
