use ricture_capture::Screenshot;
use std::path::Path;

pub(crate) fn encode_png(screenshot: &Screenshot) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, screenshot.width, screenshot.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&screenshot.rgba)?;

    Ok(bytes)
}

pub(crate) fn save_png(screenshot: &Screenshot, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, encode_png(screenshot)?)?;
    Ok(())
}
