use smithay_client_toolkit::dispatch2::Dispatch2;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::{
    wp_viewport::{self, WpViewport},
    wp_viewporter::{self, WpViewporter},
};
use wayland_client::{Connection, QueueHandle};

/// User data for `wp_viewporter` / `wp_viewport`. Neither interface emits
/// events, so the handlers are no-ops.
pub(crate) struct ViewportData;

impl<D> Dispatch2<WpViewporter, D> for ViewportData {
    fn event(
        &self,
        _: &mut D,
        _: &WpViewporter,
        _: wp_viewporter::Event,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
    }
}

impl<D> Dispatch2<WpViewport, D> for ViewportData {
    fn event(
        &self,
        _: &mut D,
        _: &WpViewport,
        _: wp_viewport::Event,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
    }
}
