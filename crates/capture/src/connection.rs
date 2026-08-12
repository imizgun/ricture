use crate::state::AppState;
use wayland_client::{Connection, EventQueue};

/// Connects, walks the registry and gives back the live connection + queue
/// plus an `AppState` with `outputs` / `shm` / `screencopy_manager` already
/// bound and ready to use.
pub(crate) fn connect() -> Result<(Connection, EventQueue<AppState>, AppState), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let mut event_queue = conn.new_event_queue::<AppState>();
    let qh = event_queue.handle();

    let display = conn.display();
    let _registry = display.get_registry(&qh, ());

    let mut state = AppState::default();

    event_queue.roundtrip(&mut state)?;
    event_queue.roundtrip(&mut state)?;

    Ok((conn, event_queue, state))
}

pub fn list_globals() -> Result<(), Box<dyn std::error::Error>> {
    connect()?;
    Ok(())
}
