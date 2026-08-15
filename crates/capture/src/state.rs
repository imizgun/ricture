use memmap2::MmapMut;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::WEnum;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

#[derive(Default)]
pub(crate) struct AppState {
    pub(crate) outputs: Vec<WlOutput>,
    pub(crate) shm: Option<WlShm>,
    pub(crate) screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    pub(crate) capture_done: bool,
    pub(crate) buffer: Option<Buffer>,
    pub(crate) wl_buffer: Option<WlBuffer>,
    pub(crate) mmap_mut: Option<MmapMut>,
}

pub(crate) struct Buffer {
    pub(crate) format: WEnum<Format>,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) stride: i32,
}

pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}