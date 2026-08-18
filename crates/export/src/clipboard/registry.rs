use super::state::ClipboardState;
use wayland_client::protocol::wl_registry::{self, Event};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;

impl Dispatch<wl_registry::WlRegistry, ()> for ClipboardState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };
        if interface == WlSeat::interface().name {
            let version = version.min(WlSeat::interface().version);
            state.seat = Some(registry.bind::<WlSeat, _, _>(name, version, qh, ()));
        } else if interface == ZwlrDataControlManagerV1::interface().name {
            let version = version.min(ZwlrDataControlManagerV1::interface().version);
            state.manager =
                Some(registry.bind::<ZwlrDataControlManagerV1, _, _>(name, version, qh, ()));
        }
    }
}

// wl_seat's own events (capabilities, name) don't matter for a clipboard-only
// client — we just need the object bound so we can pass it to
// get_data_device.
impl Dispatch<WlSeat, ()> for ClipboardState {
    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        _event: <WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

// The manager global has no events of its own.
impl Dispatch<ZwlrDataControlManagerV1, ()> for ClipboardState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlManagerV1,
        _event: <ZwlrDataControlManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
