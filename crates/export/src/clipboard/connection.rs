use super::state::ClipboardState;
use wayland_client::{Connection, EventQueue};

/// Connects, walks the registry and gives back the live connection + queue
pub(crate) fn connect()
-> Result<(Connection, EventQueue<ClipboardState>, ClipboardState), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let mut event_queue = conn.new_event_queue::<ClipboardState>();
    let qh = event_queue.handle();

    let display = conn.display();
    let _registry = display.get_registry(&qh, ());

    let mut state = ClipboardState::default();

    event_queue.roundtrip(&mut state)?;
    event_queue.roundtrip(&mut state)?;

    Ok((conn, event_queue, state))
}
