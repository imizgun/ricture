use ricture_capture::Screenshot;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let screenshot = ricture_capture::capture_first_output()?;

    let Some(((x, y, width, height), screenshot)) = ricture_overlay::run(screenshot)? else {
        return Ok(());
    };

    let cropped = crop(&screenshot, x as u32, y as u32, width as u32, height as u32);
    ricture_capture::save_png(&cropped, "screenshot.png")?;
    println!("saved screenshot.png");

    Ok(())
}

fn crop(screenshot: &Screenshot, x: u32, y: u32, width: u32, height: u32) -> Screenshot {
    let src_stride = screenshot.width as usize * 4;
    let row_bytes = width as usize * 4;

    let mut rgba = Vec::with_capacity(row_bytes * height as usize);

    for row in 0..height as usize {
        let src_offset = (y as usize + row) * src_stride + x as usize * 4;
        rgba.extend_from_slice(&screenshot.rgba[src_offset..src_offset + row_bytes]);
    }

    Screenshot { width, height, rgba }
}
