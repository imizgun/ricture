mod crop;

use chrono::Utc;
use crop::crop;
use ricture_capture::Screenshot;
use ricture_export::clipboard::copy_to_clipboard;
use ricture_export::png::{encode_png, save_png};
use ricture_config::config::Config;
use ricture_config::validate::Validate;
use ricture_overlay::{Action, state};
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>>{
    let config = Config::load()?;
    config.validate()?;
    
    let is_notification_enabled = config.general.enable_notifications;
    
    if let Err(err) = run(config) {
        eprintln!("error: {err}");

        if is_notification_enabled {
            ricture_notify::notify("ricture failed", &err.to_string());
        }
        
        std::process::exit(1);
    }
    Ok(())
}

fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let screenshot: Screenshot;
    
    screenshot = ricture_capture::capture_first_output()?;

    // if fullscreen, do only this
    if args.len() > 1 {
        if args[1] == "--fullscreen" {
            copy_to_clipboard(&encode_png(&screenshot)?)?;
            return Ok(());
        }
    }
    let save_path = PathBuf::from(
        format!(
            "{}/{}.png",
            &config.general.save_path,
            Utc::now().format("%Y-%m-%d_%H-%M-%S")
        )
        .to_string(),
    );
    let notify_enabled = config.general.enable_notifications;    
    
    let app_config = state::AppConfig::from(config);

    let Some((action, (x, y, width, height), screenshot)) =
        ricture_overlay::run(screenshot, app_config)?
    else {
        return Ok(());
    };

    let cropped = crop(&screenshot, x as u32, y as u32, width as u32, height as u32);

    match action {
        Action::Save => {
            save_png(&cropped, &save_path)?;
            println!("saved {}", save_path.display());
            if notify_enabled {
                ricture_notify::notify("Screenshot saved", &save_path.display().to_string());                
            }
        }
        Action::Copy => {
            copy_to_clipboard(&encode_png(&cropped)?)?;
            println!("copied screenshot to clipboard");

            if notify_enabled {
                ricture_notify::notify("Screenshot copied", "Image copied to clipboard");
            }
        }
    }

    Ok(())
}
