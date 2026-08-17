use crate::state::App;
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};
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
            let mut dimmed = Pixmap::new(width, height).expect("non-zero surface size");
            dimmed.data_mut().copy_from_slice(&self.screenshot.rgba);
            let full_rect = Rect::from_ltrb(0.0, 0.0, width as f32, height as f32).unwrap();
            let mut dim = Paint::default();
            dim.set_color(Color::from_rgba8(0, 0, 0, 140));
            dimmed.fill_rect(full_rect, &dim, Transform::identity(), None);
            self.renderer.dimmed = Some(dimmed);
            self.renderer.pixmap = Some(Pixmap::new(width, height).expect("non-zero surface size"));
        }
        let pixmap = self.renderer.pixmap.as_mut().unwrap();
        pixmap.data_mut().copy_from_slice(self.renderer.dimmed.as_ref().unwrap().data());

        let selection_rect = self.selection.rect();
        let prev_rect = self.selection.prev_rect();

        if let Some(rect) = selection_rect {
            undim_rect(pixmap, &self.screenshot.rgba, rect, width, height);
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

        // RGBA -> BGRA byte order
        for (dst, src) in canvas.chunks_exact_mut(4).zip(pixmap.data().chunks_exact(4)) {
            let word = u32::from_ne_bytes([src[0], src[1], src[2], src[3]]);
            let swapped = (word & 0xFF00FF00) | ((word & 0x0000_00FF) << 16) | ((word >> 16) & 0x0000_00FF);
            dst.copy_from_slice(&swapped.to_ne_bytes());
        }

        match damage_bounds(prev_rect, selection_rect, width, height) {
            Some((x, y, w, h)) if !first_draw => {
                self.layer.wl_surface().damage_buffer(x, y, w, h);
            }
            _ => {
                self.layer.wl_surface().damage_buffer(0, 0, width as i32, height as i32);
            }
        }
        self.layer.wl_surface().frame(qh, FrameCallbackData(self.layer.wl_surface().clone()));

        buffer.attach_to(self.layer.wl_surface()).expect("buffer attach");
        self.layer.commit();

        self.selection.mark_drawn();
    }
}

fn undim_rect(pixmap: &mut Pixmap, screenshot_rgba: &[u8], rect: Rect, width: u32, height: u32) {
    let x0 = rect.left().floor().max(0.0) as usize;
    let y0 = rect.top().floor().max(0.0) as usize;
    let x1 = (rect.right().ceil() as usize).min(width as usize);
    let y1 = (rect.bottom().ceil() as usize).min(height as usize);
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let stride = width as usize * 4;
    let row_bytes = (x1 - x0) * 4;
    let dst = pixmap.data_mut();
    for y in y0..y1 {
        let row_start = y * stride + x0 * 4;
        dst[row_start..row_start + row_bytes].copy_from_slice(&screenshot_rgba[row_start..row_start + row_bytes]);
    }
}

fn damage_bounds(a: Option<Rect>, b: Option<Rect>, width: u32, height: u32) -> Option<(i32, i32, i32, i32)> {
    const STROKE_PAD: f32 = 3.0;

    let expand = |r: Rect| {
        (r.left() - STROKE_PAD, r.top() - STROKE_PAD, r.right() + STROKE_PAD, r.bottom() + STROKE_PAD)
    };

    let (l, t, r, b) = match (a, b) {
        (Some(a), Some(b)) => {
            let (al, at, ar, ab) = expand(a);
            let (bl, bt, br, bb) = expand(b);
            (al.min(bl), at.min(bt), ar.max(br), ab.max(bb))
        }
        (Some(r), None) | (None, Some(r)) => expand(r),
        (None, None) => return None,
    };

    let x = l.floor().max(0.0) as i32;
    let y = t.floor().max(0.0) as i32;
    let right = r.ceil().min(width as f32) as i32;
    let bottom = b.ceil().min(height as f32) as i32;

    Some((x, y, (right - x).max(0), (bottom - y).max(0)))
}
