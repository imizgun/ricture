use crate::state::{App, ExitFlag};
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
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

        if self.first_configure {
            self.first_configure = false;
            let byte_size = self.width as usize * self.height as usize * 4;
            self.renderer.pool =
                Some(SlotPool::new(byte_size, &self.shm).expect("failed to create shm pool"));
            self.draw(qh);
        }
    }
}
