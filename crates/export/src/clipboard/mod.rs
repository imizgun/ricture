mod connection;
mod daemonize;
mod registry;
mod source;
mod state;

pub fn copy_to_clipboard(png: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let (_conn, mut eq, mut state) = connection::connect()?;
    let qh = eq.handle();

    let seat = state.seat.as_ref().ok_or("compositor has no wl_seat")?;
    let manager = state
        .manager
        .as_ref()
        .ok_or("compositor doesn't support zwlr_data_control_manager_v1")?;

    let device = manager.get_data_device(seat, &qh, ());
    let source = manager.create_data_source(&qh, ());

    source.offer("image/png".to_string());
    device.set_selection(Some(&source));
    state.payload = png.to_vec();

    eq.roundtrip(&mut state)?;

    if !daemonize::daemonize() {
        // Parent: selection is already live, nothing left to do here.
        return Ok(());
    }

    // Child: detached from the terminal, keep serving Send/Cancelled until selection ownership moves on.
    while !state.done {
        eq.blocking_dispatch(&mut state)?;
    }

    Ok(())
}
