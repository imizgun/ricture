use crate::state::App;
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

impl App {
    pub(crate) fn draw(&mut self, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = width as i32 * 4;

        let pool = self.pool.as_mut().expect("pool is created on first configure");
        let (buffer, canvas) = pool
            .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
            .expect("create buffer");

        // TODO(you): placeholder — flat, half-transparent dark fill so you
        // can see the surface is actually there and sized correctly.
        // Next step: composite the frozen ricture-capture screenshot here
        // (e.g. with tiny-skia) instead of this solid color, then draw the
        // selection rectangle + dim mask on top once pointer.rs tracks one.
        for chunk in canvas.chunks_exact_mut(4) {
            let pixel: &mut [u8; 4] = chunk.try_into().unwrap();
            *pixel = 0x80202020u32.to_le_bytes();
        }

        self.layer.wl_surface().damage_buffer(0, 0, width as i32, height as i32);
        self.layer.wl_surface().frame(qh, FrameCallbackData(self.layer.wl_surface().clone()));

        buffer.attach_to(self.layer.wl_surface()).expect("buffer attach");
        self.layer.commit();
    }
}
