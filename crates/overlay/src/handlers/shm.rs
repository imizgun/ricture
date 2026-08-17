use crate::state::App;
use smithay_client_toolkit::shm::{Shm, ShmHandler};

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}
