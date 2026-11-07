use slint::PlatformError;
use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::platform::{Platform, WindowAdapter};
use std::rc::Rc;

/// Minimal Slint platform: hands out our single software window. The event
/// loop is ours (the Wayland queue), so Slint's own `run_event_loop` is unused.
struct OverlayPlatform {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for OverlayPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }
}

/// Installs the platform. Slint allows this once per process, which matches
/// the overlay's lifetime.
pub(super) fn install(window: Rc<MinimalSoftwareWindow>) -> Result<(), PlatformError> {
    slint::platform::set_platform(Box::new(OverlayPlatform { window }))
        .map_err(|err| PlatformError::Other(err.to_string()))
}
