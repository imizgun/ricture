use smithay_client_toolkit::shm::slot::SlotPool;
use tiny_skia::{Mask, Pixmap, Rect};

#[derive(Default)]
pub(crate) struct Renderer {
    /// Composed frame, uploaded to the shm buffer.
    pub(crate) pixmap: Option<Pixmap>,
    /// Screenshot with committed annotations flattened onto it.
    pub(crate) base: Option<Pixmap>,
    /// `base` with the dim layer on top.
    pub(crate) dimmed: Option<Pixmap>,
    /// `Document::revision()` that `base`/`dimmed` were built from.
    pub(crate) doc_revision: Option<u64>,
    /// Clip mask for live gestures, keyed by the selection it was built for.
    pub(crate) clip: Option<(Rect, Mask)>,
    pub(crate) pool: Option<SlotPool>,
}
