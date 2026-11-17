//! The overlay darkens everything outside the selection. Computed while
//! composing each frame instead of cached: a cached dimmed copy costs a
//! screen-sized buffer (memory, and page faults at startup), and this pass
//! runs about as fast as copying such a copy would.

/// How much of each channel survives: the result of drawing black with this
/// alpha over the screenshot.
const DIM_ALPHA: u16 = 140;
const KEEP: u16 = 255 - DIM_ALPHA;

/// Per-byte addend for one 16-byte (4-pixel) block: alpha also gains the
/// black layer's own coverage, as in source-over.
const ADD: [u16; 16] = [0, 0, 0, DIM_ALPHA, 0, 0, 0, DIM_ALPHA, 0, 0, 0, DIM_ALPHA, 0, 0, 0, DIM_ALPHA];

const BYTES_PER_PIXEL: usize = 4;

/// Writes `base` into `dst` (both premultiplied RGBA, `width` px wide),
/// dimmed everywhere except the `undimmed` pixel rect `(x0, y0, x1, y1)`.
pub(crate) fn compose_dimmed(
    dst: &mut [u8],
    base: &[u8],
    width: usize,
    undimmed: Option<(usize, usize, usize, usize)>,
) {
    let stride = width * BYTES_PER_PIXEL;
    let (x0, y0, x1, y1) = undimmed
        .filter(|&(x0, y0, x1, y1)| x0 < x1 && y0 < y1)
        .map_or((0, 0, 0, 0), |(x0, y0, x1, y1)| (x0, y0, x1.min(width), y1));
    let (left, right) = (x0 * BYTES_PER_PIXEL, x1 * BYTES_PER_PIXEL);

    for (y, (d, s)) in dst.chunks_exact_mut(stride).zip(base.chunks_exact(stride)).enumerate() {
        if y < y0 || y >= y1 {
            dim_row(d, s);
            continue;
        }
        dim_row(&mut d[..left], &s[..left]);
        d[left..right].copy_from_slice(&s[left..right]);
        dim_row(&mut d[right..], &s[right..]);
    }
}

/// Fixed 16-byte blocks of u16 math, so the compiler turns this into SIMD
/// even for baseline x86-64 (SSE2).
fn dim_row(dst: &mut [u8], src: &[u8]) {
    let mut dst_blocks = dst.chunks_exact_mut(16);
    let mut src_blocks = src.chunks_exact(16);
    for (d, s) in (&mut dst_blocks).zip(&mut src_blocks) {
        let d: &mut [u8; 16] = d.try_into().unwrap();
        let s: &[u8; 16] = s.try_into().unwrap();
        for i in 0..16 {
            d[i] = dim_byte(s[i], ADD[i]);
        }
    }
    let (d, s) = (dst_blocks.into_remainder(), src_blocks.remainder());
    for i in 0..d.len() {
        d[i] = dim_byte(s[i], ADD[i]);
    }
}

#[inline(always)]
fn dim_byte(value: u8, add: u16) -> u8 {
    // Rounded value * KEEP / 255; fits u16 (255 * 115 + 128 < 65536).
    let v = value as u16 * KEEP + 128;
    (((v + (v >> 8)) >> 8) + add) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

    /// Every byte value at every channel position, plus a ragged tail.
    fn sample(width: usize, height: usize) -> Vec<u8> {
        (0..width * height * BYTES_PER_PIXEL).map(|i| (i * 7 % 256) as u8).collect()
    }

    #[test]
    fn matches_drawing_translucent_black_over_it() {
        let (width, height) = (37, 3);
        let mut base = sample(width, height);
        // Premultiplied: colour can't exceed alpha. Opaque, like a capture.
        for px in base.chunks_exact_mut(4) {
            px[3] = 255;
        }

        let mut expected = Pixmap::from_vec(base.clone(), tiny_skia::IntSize::from_wh(width as u32, height as u32).unwrap()).unwrap();
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(0, 0, 0, DIM_ALPHA as u8));
        let full = Rect::from_xywh(0.0, 0.0, width as f32, height as f32).unwrap();
        expected.fill_rect(full, &paint, Transform::identity(), None);

        let mut dst = vec![0; base.len()];
        compose_dimmed(&mut dst, &base, width, None);
        for (i, (&got, &want)) in dst.iter().zip(expected.data()).enumerate() {
            assert!(got.abs_diff(want) <= 1, "byte {i}: {got} vs {want}");
        }
    }

    #[test]
    fn leaves_the_undimmed_rect_as_is() {
        let (width, height) = (9, 4);
        let base = sample(width, height);
        let mut dst = vec![0; base.len()];
        compose_dimmed(&mut dst, &base, width, Some((2, 1, 5, 3)));

        let at = |x: usize, y: usize| (y * width + x) * BYTES_PER_PIXEL;
        for y in 0..height {
            for x in 0..width {
                let i = at(x, y);
                let inside = (2..5).contains(&x) && (1..3).contains(&y);
                let px = &dst[i..i + 4];
                if inside {
                    assert_eq!(px, &base[i..i + 4], "({x}, {y}) should be untouched");
                } else {
                    assert_eq!(px[0], dim_byte(base[i], 0), "({x}, {y}) should be dimmed");
                    assert_eq!(px[3], dim_byte(base[i + 3], DIM_ALPHA));
                }
            }
        }
    }

    #[test]
    fn dims_opaque_white_and_black() {
        assert_eq!(dim_byte(255, 0), 115);
        assert_eq!(dim_byte(0, 0), 0);
        assert_eq!(dim_byte(255, DIM_ALPHA), 255);
    }
}
