use crate::state::AppState;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

impl Dispatch<WlOutput, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &WlOutput,
        _event: <WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}
