use crate::state::AppState;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_registry::{self, Event};
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let Event::Global { name, interface, version } = event else {
            return;
        };
        if interface == WlOutput::interface().name {
            let version = version.min(WlOutput::interface().version);
            state.outputs.push(registry.bind::<WlOutput, _, _>(name, version, qh, ()));
        } else if interface == WlShm::interface().name {
            let version = version.min(WlShm::interface().version);
            state.shm = Some(registry.bind::<WlShm, _, _>(name, version, qh, ()));
        } else if interface == ZwlrScreencopyManagerV1::interface().name {
            let version = version.min(ZwlrScreencopyManagerV1::interface().version);
            state.screencopy_manager =
                Some(registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, qh, ()));
        }
    }
}
