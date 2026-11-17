use crate::frame_buffer::alloc_frame;
use crate::state::{Buffer, Screenshot};
use wayland_client::protocol::wl_shm::Format;

const BYTES_PER_PIXEL: usize = 4;

/// Converts a wl_shm `Argb8888`/`Xrgb8888` frame (B,G,R,A/X byte order in
/// memory) into a plain RGBA8 `Screenshot`.
pub(crate) fn to_rgba(
    buffer: &Buffer,
    pixels: &[u8],
) -> Result<Screenshot, Box<dyn std::error::Error>> {
    let width = buffer.width as usize;
    let height = buffer.height as usize;
    let stride = buffer.stride as usize;
    let alpha = match buffer.format.into_result()? {
        Format::Argb8888 => 0,
        // The X byte is undefined; force opaque.
        Format::Xrgb8888 => 0xFF00_0000,
        other => return Err(format!("unsupported shm format: {other:?}").into()),
    };

    let row_bytes = width * BYTES_PER_PIXEL;
    let mut rgba = alloc_frame(row_bytes * height);
    for (dst_row, src_row) in rgba.chunks_exact_mut(row_bytes).zip(pixels.chunks_exact(stride)) {
        let src_row = &src_row[..row_bytes];
        for (dst, src) in dst_row.chunks_exact_mut(BYTES_PER_PIXEL).zip(src_row.chunks_exact(BYTES_PER_PIXEL)) {
            // Little endian u32 of B,G,R,A bytes: swap the B and R bytes.
            let word = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
            let swapped =
                (word & 0xFF00_FF00) | ((word & 0xFF) << 16) | ((word >> 16) & 0xFF) | alpha;
            dst.copy_from_slice(&swapped.to_le_bytes());
        }
    }

    Ok(Screenshot {
        width: buffer.width as u32,
        height: buffer.height as u32,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wayland_client::WEnum;

    fn buffer(format: Format, width: i32, height: i32, stride: i32) -> Buffer {
        Buffer { format: WEnum::Value(format), width, height, stride }
    }

    #[test]
    fn swaps_to_rgba_and_skips_row_padding() {
        // 2x2 pixels, rows padded to 12 bytes.
        let pixels = [
            1, 2, 3, 4, 5, 6, 7, 8, 0xEE, 0xEE, 0xEE, 0xEE, //
            9, 10, 11, 12, 13, 14, 15, 16, 0xEE, 0xEE, 0xEE, 0xEE,
        ];
        let shot = to_rgba(&buffer(Format::Argb8888, 2, 2, 12), &pixels).unwrap();
        assert_eq!((shot.width, shot.height), (2, 2));
        assert_eq!(shot.rgba, [3, 2, 1, 4, 7, 6, 5, 8, 11, 10, 9, 12, 15, 14, 13, 16]);
    }

    #[test]
    fn xrgb_is_made_opaque() {
        let shot = to_rgba(&buffer(Format::Xrgb8888, 1, 1, 4), &[1, 2, 3, 0]).unwrap();
        assert_eq!(shot.rgba, [3, 2, 1, 255]);
    }

    #[test]
    fn other_formats_are_rejected() {
        assert!(to_rgba(&buffer(Format::Rgb565, 1, 1, 4), &[0; 4]).is_err());
    }
}
