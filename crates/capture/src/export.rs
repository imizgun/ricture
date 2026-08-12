use crate::state::Buffer;
use std::fs::File;
use std::io::BufWriter;
use wayland_client::protocol::wl_shm::Format;

/// Converts a wl_shm `Argb8888`/`Xrgb8888` frame (B,G,R,A/X byte order in
/// memory) into an RGBA PNG on disk.
pub(crate) fn write_png(path: &str, buffer: &Buffer, pixels: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let format = buffer.format.into_result()?;
    let width = buffer.width as usize;
    let height = buffer.height as usize;
    let stride = buffer.stride as usize;

    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let row_start = row * stride;
        for col in 0..width {
            let px = row_start + col * 4;
            let (b, g, r) = (pixels[px], pixels[px + 1], pixels[px + 2]);
            let a = match format {
                Format::Argb8888 => pixels[px + 3],
                Format::Xrgb8888 => 255,
                other => return Err(format!("unsupported shm format: {other:?}").into()),
            };
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }

    let writer = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(writer, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgba)?;

    Ok(())
}
