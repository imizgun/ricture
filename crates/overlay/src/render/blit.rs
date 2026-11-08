use tiny_skia::Pixmap;

const BYTES_PER_PIXEL: usize = 4;

/// Runs of non-transparent pixels in a mostly empty, screen-sized layer, so
/// compositing it costs only what is visible instead of a full-screen blend.
#[derive(Default)]
pub(crate) struct VisibleRuns {
    runs: Vec<Run>,
}

/// A horizontal run, in bytes into the pixmap data.
struct Run {
    start: usize,
    len: usize,
}

impl VisibleRuns {
    /// Rescans `layer`; call whenever its contents change.
    pub(crate) fn rebuild(&mut self, layer: &Pixmap) {
        self.runs.clear();
        let stride = layer.width() as usize * BYTES_PER_PIXEL;
        for (y, row) in layer.data().chunks_exact(stride).enumerate() {
            let row_start = y * stride;
            let mut run_start = None;
            for (x, pixel) in row.chunks_exact(BYTES_PER_PIXEL).enumerate() {
                let offset = x * BYTES_PER_PIXEL;
                match (pixel[3] != 0, run_start) {
                    (true, None) => run_start = Some(offset),
                    (false, Some(start)) => {
                        self.runs.push(Run {
                            start: row_start + start,
                            len: offset - start,
                        });
                        run_start = None;
                    }
                    _ => {}
                }
            }
            if let Some(start) = run_start {
                self.runs.push(Run {
                    start: row_start + start,
                    len: stride - start,
                });
            }
        }
    }

    /// Source-over of `layer` onto `dst`. Both are premultiplied RGBA of the
    /// same size, and the runs must come from `layer`'s current contents.
    pub(crate) fn composite(&self, layer: &Pixmap, dst: &mut Pixmap) {
        let src = layer.data();
        let dst = dst.data_mut();
        for run in &self.runs {
            let range = run.start..run.start + run.len;
            let pixels = dst[range.clone()]
                .chunks_exact_mut(BYTES_PER_PIXEL)
                .zip(src[range].chunks_exact(BYTES_PER_PIXEL));
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
