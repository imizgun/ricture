use smithay_client_toolkit::shm::slot::SlotPool;
use tiny_skia::Pixmap;

#[derive(Default)]
pub(crate) struct Renderer {
    /// Composed frame, uploaded to the shm buffer.
    pub(crate) pixmap: Option<Pixmap>,
    /// Screenshot with committed annotations flattened onto it. `None` while
    /// there are none: the screenshot itself serves then.
    pub(crate) base: Option<Pixmap>,
    /// `Document::revision()` that `base` was built from.
    pub(crate) doc_revision: Option<u64>,
    pub(crate) pool: Option<SlotPool>,
}
