use smithay_client_toolkit::shm::slot::SlotPool;
use tiny_skia::Pixmap;

#[derive(Default)]
pub(crate) struct Renderer {
    pub(crate) pixmap: Option<Pixmap>,
    pub(crate) pool: Option<SlotPool>,
}
