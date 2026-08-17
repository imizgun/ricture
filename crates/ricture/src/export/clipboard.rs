use std::io::Write;
use std::process::{Command, Stdio};

// Shells out to `wl-copy` (wl-clipboard) rather than speaking
// wl_data_device_manager ourselves — a native implementation would need to
// keep this process alive after the overlay closes to answer `Send`
// requests, which is a bigger lifecycle change than just this function.
pub(crate) fn copy_to_clipboard(png: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut child =
        Command::new("wl-copy").arg("--type").arg("image/png").stdin(Stdio::piped()).spawn()?;
    child.stdin.take().expect("stdin was piped").write_all(png)?;
    child.wait()?;
    Ok(())
}
