use ricture_capture::Screenshot;
use ricture_config::config::Config;
use ricture_config::validate::Validate;
use ricture_overlay::{Action, state};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    config.validate()?;
    let save_path = PathBuf::from(format!("{}/{}.png", &config.general.save_path, Utc::now().format("%Y-%m-%d_%H-%M-%S")).to_string());
    let app_config = state::AppConfig::from(config);

    let screenshot = ricture_capture::capture_first_output()?;

    let Some((action, (x, y, width, height), screenshot)) = ricture_overlay::run(screenshot, app_config)? else {
        return Ok(());
    };

    let cropped = crop(&screenshot, x as u32, y as u32, width as u32, height as u32);

    match action {
        Action::Save => {
            ricture_capture::save_png(&cropped, &save_path)?;
            println!("saved {}", save_path.display());
        }
        Action::Copy => {
            copy_to_clipboard(&ricture_capture::encode_png(&cropped)?)?;
            println!("copied screenshot to clipboard");
        }
    }

    Ok(())
}

fn copy_to_clipboard(png: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut child =
        Command::new("wl-copy").arg("--type").arg("image/png").stdin(Stdio::piped()).spawn()?;
    child.stdin.take().expect("stdin was piped").write_all(png)?;
    child.wait()?;
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
