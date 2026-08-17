use ricture_capture::Screenshot;

pub(crate) fn crop(screenshot: &Screenshot, x: u32, y: u32, width: u32, height: u32) -> Screenshot {
    let src_stride = screenshot.width as usize * 4;
    let row_bytes = width as usize * 4;

    let mut rgba = Vec::with_capacity(row_bytes * height as usize);

    for row in 0..height as usize {
        let src_offset = (y as usize + row) * src_stride + x as usize * 4;
        rgba.extend_from_slice(&screenshot.rgba[src_offset..src_offset + row_bytes]);
    }

    Screenshot { width, height, rgba }
}
