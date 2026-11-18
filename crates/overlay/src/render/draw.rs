use crate::render::dim::{compose_dimmed, dim_outside};
use crate::state::App;
use ricture_capture::{Screenshot, alloc_frame};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use tiny_skia::{IntSize, Paint, PathBuilder, Pixmap, PixmapMut, Rect, Stroke, Transform};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

impl App {
    /// Composes and commits a frame:
    ///
    /// ```text
    /// base (screenshot + annotations) + the live gesture,
    ///   dimmed outside the selection
    ///   + selection border
    ///   + Slint UI layer
    /// ```
    ///
    /// Nothing is clipped to the selection: annotations may be drawn past it
    /// and are dimmed like the screenshot under them, the same before and
    /// after the gesture is committed. The export crops to the selection.
    pub(crate) fn draw(&mut self, qh: &QueueHandle<Self>) {
        // Draw in screenshot (physical) pixels; the viewport maps this buffer
        // onto the logical surface size.
        let width = self.screenshot.width;
        let height = self.screenshot.height;
        let stride = width as i32 * 4;

        self.sync_ui();

        let first_draw = self.renderer.pixmap.is_none();
        let selection_changed = self.selection.changed();
        let doc_changed = self.renderer.doc_revision != Some(self.document.revision());
        let gesture_changed = std::mem::take(&mut self.canvas_dirty);
        let ui_changed = self.ui.as_mut().is_some_and(|ui| ui.render());

        if !first_draw && !selection_changed && !doc_changed && !gesture_changed && !ui_changed {
            self.layer
                .wl_surface()
                .frame(qh, FrameCallbackData(self.layer.wl_surface().clone()));
            self.layer.commit();
            return;
        }

        let _frame = ricture_profile::span("frame");
        if doc_changed {
            let _span = ricture_profile::span("frame: rebuild base");
            self.rebuild_base(width, height);
        }
        let compose = ricture_profile::span("frame: compose");
        if first_draw {
            self.renderer.pixmap = Some(frame_pixmap(width, height, None));
        }

        let selection_rect = self.selection.rect();
        let prev_rect = self.selection.prev_rect();

        let pixmap = self.renderer.pixmap.as_mut().unwrap();
        let base = self
            .renderer
            .base
            .as_ref()
            .map_or(self.screenshot.rgba.as_slice(), |base| base.data());
        let undimmed = selection_rect.and_then(|rect| pixel_bounds(rect, width, height));
        if self.document.is_drawing() {
            // Two passes, because the gesture has to be in place before the
            // dimming runs over it.
            pixmap.data_mut().copy_from_slice(base);
            self.document.render_active(&mut pixmap.as_mut());
            dim_outside(pixmap.data_mut(), width as usize, undimmed);
        } else {
            compose_dimmed(pixmap.data_mut(), base, width as usize, undimmed);
        }

        if let Some(rect) = selection_rect {
            let border = PathBuilder::from_rect(rect);
            let mut paint = Paint::default();
            paint.set_color(self.config.rect_color);
            let stroke = Stroke {
                width: 2.0,
                ..Default::default()
            };
            pixmap.stroke_path(&border, &paint, &stroke, Transform::identity(), None);
        }

        if let Some(ui) = &self.ui {
            ui.composite_onto(pixmap);
        }
        drop(compose);
        let to_shm = ricture_profile::span("frame: to shm (bgra)");

        let pool = self
            .renderer
            .pool
            .as_mut()
            .expect("pool is created on first configure");
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("create buffer");

        // RGBA -> BGRA byte order
        for (dst, src) in canvas
            .chunks_exact_mut(4)
            .zip(pixmap.data().chunks_exact(4))
        {
            let word = u32::from_ne_bytes([src[0], src[1], src[2], src[3]]);
            let swapped =
                (word & 0xFF00FF00) | ((word & 0x0000_00FF) << 16) | ((word >> 16) & 0x0000_00FF);
            dst.copy_from_slice(&swapped.to_ne_bytes());
        }

        drop(to_shm);

        // Only a selection change is cheap to bound; anything else (UI,
        // annotations) repaints the whole buffer.
        let only_selection = !first_draw && !doc_changed && !gesture_changed && !ui_changed;
        match damage_bounds(prev_rect, selection_rect, width, height) {
            Some((x, y, w, h)) if only_selection => {
                self.layer.wl_surface().damage_buffer(x, y, w, h);
            }
            _ => {
                self.layer
                    .wl_surface()
                    .damage_buffer(0, 0, width as i32, height as i32);
            }
        }
        self.layer
            .wl_surface()
            .frame(qh, FrameCallbackData(self.layer.wl_surface().clone()));

        buffer
            .attach_to(self.layer.wl_surface())
            .expect("buffer attach");
        self.layer.commit();

        self.selection.mark_drawn();
    }

    /// Re-flattens committed annotations onto the screenshot.
    fn rebuild_base(&mut self, width: u32, height: u32) {
        self.renderer.base = (!self.document.is_empty()).then(|| {
            // The capture is opaque, so straight and premultiplied RGBA coincide.
            let mut base = frame_pixmap(width, height, Some(&self.screenshot.rgba));
            self.document.render_committed(&mut base.as_mut());
            base
        });
        self.renderer.doc_revision = Some(self.document.revision());
    }

    /// The final image: the screenshot with all annotations applied.
    pub(crate) fn flatten(mut self) -> Screenshot {
        let (width, height) = (self.screenshot.width, self.screenshot.height);
        if let Some(mut canvas) = PixmapMut::from_bytes(&mut self.screenshot.rgba, width, height) {
            self.document.render_committed(&mut canvas);
        }
        self.screenshot
    }
}

/// A screen-sized pixmap in a huge-page backed buffer, see `alloc_frame`.
fn frame_pixmap(width: u32, height: u32, contents: Option<&[u8]>) -> Pixmap {
    let mut data = alloc_frame(width as usize * height as usize * 4);
    if let Some(contents) = contents {
        data.copy_from_slice(contents);
    }
    let size = IntSize::from_wh(width, height).expect("non-zero surface size");
    Pixmap::from_vec(data, size).expect("buffer matches its size")
}

/// Whole pixels covered by `rect`, clipped to the buffer, as `(x0, y0, x1, y1)`.
fn pixel_bounds(rect: Rect, width: u32, height: u32) -> Option<(usize, usize, usize, usize)> {
    let x0 = rect.left().floor().max(0.0) as usize;
    let y0 = rect.top().floor().max(0.0) as usize;
    let x1 = (rect.right().ceil() as usize).min(width as usize);
    let y1 = (rect.bottom().ceil() as usize).min(height as usize);
    (x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
}

fn damage_bounds(
    a: Option<Rect>,
    b: Option<Rect>,
    width: u32,
    height: u32,
) -> Option<(i32, i32, i32, i32)> {
    const STROKE_PAD: f32 = 3.0;

    let expand = |r: Rect| {
        (
            r.left() - STROKE_PAD,
            r.top() - STROKE_PAD,
            r.right() + STROKE_PAD,
            r.bottom() + STROKE_PAD,
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use ricture_tools::{Brush, Document, Style};
    use tiny_skia::{Color, Point};

    const WIDTH: usize = 60;
    const HEIGHT: usize = 40;
    /// Half the canvas, so a stroke across it falls on both sides.
    const SELECTION: (usize, usize, usize, usize) = (20, 5, 45, 35);

    fn screenshot() -> Vec<u8> {
        vec![255; WIDTH * HEIGHT * 4]
    }

    fn render(pixels: &mut [u8], draw: impl FnOnce(&mut PixmapMut<'_>)) {
        let mut canvas = PixmapMut::from_bytes(pixels, WIDTH as u32, HEIGHT as u32).unwrap();
        draw(&mut canvas);
    }

    /// The whole point of composing the way `draw` does: a stroke that runs
    /// past the selection must look the same while it is drawn as it does
    /// once the button comes up. Clipping the live gesture to the selection
    /// is what used to make the part outside appear only on release.
    #[test]
    fn a_stroke_crossing_the_selection_survives_the_button_coming_up() {
        let mut document = Document::default();
        let style = Style { color: Color::from_rgba8(255, 0, 0, 255), width: 6.0 };
        document.begin(&Brush, Point::from_xy(5.0, 20.0), style);
        for x in (8..=55).step_by(2) {
            document.update(Point::from_xy(x as f32, 20.0));
        }

        // While drawing: the gesture goes onto the screenshot, then the
        // dimming runs over both.
        let mut live = screenshot();
        render(&mut live, |canvas| document.render_active(canvas));
        dim_outside(&mut live, WIDTH, Some(SELECTION));

        // Committed: the gesture is part of the base `compose_dimmed` reads.
        document.finish();
        let mut base = screenshot();
        render(&mut base, |canvas| document.render_committed(canvas));
        let mut committed = vec![0; base.len()];
        compose_dimmed(&mut committed, &base, WIDTH, Some(SELECTION));

        assert_eq!(live, committed, "the frame changed when the gesture was committed");

        // And the stroke really is on both sides of the selection's edge.
        let mut empty = vec![0; base.len()];
        compose_dimmed(&mut empty, &screenshot(), WIDTH, Some(SELECTION));
        let at = |x: usize, y: usize| (y * WIDTH + x) * 4;
        for x in [8, 30, 52] {
            assert_ne!(live[at(x, 20)..at(x, 20) + 4], empty[at(x, 20)..at(x, 20) + 4], "at x={x}");
        }
    }
}
