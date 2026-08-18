use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;

#[derive(Default)]
pub(crate) struct ClipboardState {
    pub(crate) seat: Option<WlSeat>,
    pub(crate) manager: Option<ZwlrDataControlManagerV1>,

    /// The bytes offering as clipboard owner
    pub(crate) payload: Vec<u8>,
    pub(crate) done: bool,
}
