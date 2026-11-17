use tiny_skia::Pixmap;

const BYTES_PER_PIXEL: usize = 4;

/// Runs of non-transparent pixels in a mostly empty layer, so compositing it
/// costs only what is visible instead of a blend of the whole layer.
#[derive(Default)]
pub(crate) struct VisibleRuns {
    runs: Vec<Run>,
}

/// A horizontal run in the layer, in pixels.
struct Run {
    y: usize,
    x: usize,
    len: usize,
}

impl VisibleRuns {
    /// Rescans `layer`; call whenever its contents change.
    pub(crate) fn rebuild(&mut self, layer: &Pixmap) {
        self.runs.clear();
        let width = layer.width() as usize;
        for (y, row) in layer.data().chunks_exact(width * BYTES_PER_PIXEL).enumerate() {
            let mut run_start = None;
            for (x, pixel) in row.chunks_exact(BYTES_PER_PIXEL).enumerate() {
                match (pixel[3] != 0, run_start) {
                    (true, None) => run_start = Some(x),
                    (false, Some(start)) => {
                        self.runs.push(Run { y, x: start, len: x - start });
                        run_start = None;
                    }
                    _ => {}
                }
            }
            if let Some(start) = run_start {
                self.runs.push(Run { y, x: start, len: width - start });
            }
        }
    }

    /// Source-over of `layer` onto `dst` with the layer's top-left corner at
    /// `origin`. Both are premultiplied RGBA, the layer must fit inside `dst`
    /// there, and the runs must come from the layer's current contents.
    pub(crate) fn composite(&self, layer: &Pixmap, dst: &mut Pixmap, (ox, oy): (u32, u32)) {
        let layer_width = layer.width() as usize;
        let dst_width = dst.width() as usize;
        let (ox, oy) = (ox as usize, oy as usize);
        debug_assert!(ox + layer_width <= dst_width);
        debug_assert!(oy + layer.height() as usize <= dst.height() as usize);

        let src = layer.data();
        let dst = dst.data_mut();
        for run in &self.runs {
            let src_start = (run.y * layer_width + run.x) * BYTES_PER_PIXEL;
            let dst_start = ((oy + run.y) * dst_width + ox + run.x) * BYTES_PER_PIXEL;
            let bytes = run.len * BYTES_PER_PIXEL;
            let pixels = dst[dst_start..dst_start + bytes]
                .chunks_exact_mut(BYTES_PER_PIXEL)
                .zip(src[src_start..src_start + bytes].chunks_exact(BYTES_PER_PIXEL));
            for (d, s) in pixels {
                let inverse_alpha = 255 - s[3] as u32;
                for (d, s) in d.iter_mut().zip(s) {
                    // Saturating: rounding can leave a colour slightly above alpha.
                    *d = s.saturating_add(div255(*d as u32 * inverse_alpha));
                }
            }
        }
    }
}

/// Rounded division by 255 for values up to 255 * 255.
fn div255(value: u32) -> u8 {
    let value = value + 128;
    ((value + (value >> 8)) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::{Color, PremultipliedColorU8};

    fn pixel(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
        let c = pixmap.pixel(x, y).unwrap();
        [c.red(), c.green(), c.blue(), c.alpha()]
    }

    #[test]
    fn composites_layer_at_its_origin() {
        let mut layer = Pixmap::new(3, 2).unwrap();
        let opaque = PremultipliedColorU8::from_rgba(10, 20, 30, 255).unwrap();
        layer.pixels_mut()[1] = opaque; // (1, 0)
        layer.pixels_mut()[5] = opaque; // (2, 1), ends a row
        let mut runs = VisibleRuns::default();
        runs.rebuild(&layer);

        let mut dst = Pixmap::new(6, 4).unwrap();
        dst.fill(Color::from_rgba8(200, 200, 200, 255));
        runs.composite(&layer, &mut dst, (2, 1));

        assert_eq!(pixel(&dst, 3, 1), [10, 20, 30, 255]);
        assert_eq!(pixel(&dst, 4, 2), [10, 20, 30, 255]);
        // Transparent layer pixels and everything outside are untouched.
        assert_eq!(pixel(&dst, 2, 1), [200, 200, 200, 255]);
        assert_eq!(pixel(&dst, 1, 1), [200, 200, 200, 255]);
        assert_eq!(pixel(&dst, 5, 2), [200, 200, 200, 255]);
    }
}
