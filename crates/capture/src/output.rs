use crate::state::AppState;
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

impl Dispatch<WlOutput, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Geometry { x, y, .. } => {
                println!("x = {}, y = {}", x, y);
            }
            wl_output::Event::Mode { width, height, refresh, .. } => {
                println!("width: {}, height: {}, refresh: {}", width, height, refresh)
            }
            wl_output::Event::Name { name } => {
                println!("name: {}", name);
            }
            wl_output::Event::Description { description } => {
                println!("description: {}", description);
            }
            _ => println!("{:?}", event),
        }
    }
}
