mod crop;
mod export;

use chrono::Utc;
use crop::crop;
use export::clipboard::copy_to_clipboard;
use export::png::{encode_png, save_png};
use ricture_capture::Screenshot;
use ricture_config::config::Config;
use ricture_config::validate::Validate;
use ricture_overlay::{Action, state};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args = std::env::args().collect::<Vec<_>>();
    let screenshot: Screenshot;
    
    screenshot = ricture_capture::capture_first_output()?;

    // if fullscreen, do only this
    if args.len() > 1 {
        println!("{:?}", args);
        if args[1] == "--fullscreen" {
            copy_to_clipboard(&encode_png(&screenshot)?)?;
            return Ok(());
        }
    }
    
    let config = Config::load()?;
    config.validate()?;
    
    let save_path = PathBuf::from(
        format!(
            "{}/{}.png",
            &config.general.save_path,
            Utc::now().format("%Y-%m-%d_%H-%M-%S")
        )
        .to_string(),
    );
    
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
        }
        Action::Copy => {
            copy_to_clipboard(&encode_png(&cropped)?)?;
            println!("copied screenshot to clipboard");
        }
    }

    Ok(())
}

// TODO:
// 5. native clipboard support
