use crate::state::App;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::{delegate_registry, registry_handlers};

delegate_registry!(App);

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![smithay_client_toolkit::output::OutputState, smithay_client_toolkit::seat::SeatState];
}

smithay_client_toolkit::delegate_dispatch2!(App);
