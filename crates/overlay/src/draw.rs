use crate::state::App;
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

impl App {
    pub(crate) fn draw(&mut self, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = width as i32 * 4;

        let first_draw = self.renderer.pixmap.is_none();
        let selection_changed = self.selection.changed();

        if !first_draw && !selection_changed {
            self.layer.wl_surface().frame(qh, FrameCallbackData(self.layer.wl_surface().clone()));
            self.layer.commit();
            return;
        }

        if first_draw {
            self.renderer.pixmap = Some(Pixmap::new(width, height).expect("non-zero surface size"));
        }
        let pixmap = self.renderer.pixmap.as_mut().unwrap();
        pixmap.data_mut().copy_from_slice(&self.screenshot.rgba);

        let selection_rect = self.selection.rect();

        let mut mask = PathBuilder::new();
        mask.push_rect(Rect::from_ltrb(0.0, 0.0, width as f32, height as f32).unwrap());
        if let Some(rect) = selection_rect {
            mask.push_rect(rect);
        }
        if let Some(mask) = mask.finish() {
            let mut dim = Paint::default();
            dim.set_color(Color::from_rgba8(0, 0, 0, 140));
            pixmap.fill_path(&mask, &dim, FillRule::EvenOdd, Transform::identity(), None);
        }

        if let Some(rect) = selection_rect {
            let border = PathBuilder::from_rect(rect);
            let mut paint = Paint::default();
            paint.set_color(self.config.rect_color);
            let stroke = Stroke { width: 2.0, ..Default::default() };
            pixmap.stroke_path(&border, &paint, &stroke, Transform::identity(), None);
        }

        let pool = self.renderer.pool.as_mut().expect("pool is created on first configure");
        let (buffer, canvas) = pool
            .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
            .expect("create buffer");

        for (dst, src) in canvas.chunks_exact_mut(4).zip(pixmap.data().chunks_exact(4)) {
            dst[0] = src[2]; // B ← R
            dst[1] = src[1]; // G ← G
            dst[2] = src[0]; // R ← B
            dst[3] = src[3]; // A ← A
        }

        self.layer.wl_surface().damage_buffer(0, 0, width as i32, height as i32);
        self.layer.wl_surface().frame(qh, FrameCallbackData(self.layer.wl_surface().clone()));

        buffer.attach_to(self.layer.wl_surface()).expect("buffer attach");
        self.layer.commit();

        self.selection.mark_drawn();
    }
}
