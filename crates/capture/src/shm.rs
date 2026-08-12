use crate::state::AppState;
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use std::os::fd::AsFd;
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_shm::{self, Format, WlShm};
use wayland_client::protocol::wl_shm_pool::{self, WlShmPool};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

/// Allocates an anonymous shm-backed `wl_buffer` of the given geometry and
/// hands back the buffer proxy plus a writable mapping of its memory.
///
/// `stride` and `format` should come straight from whatever advertised them
/// (e.g. a screencopy frame's `buffer` event) rather than being recomputed
/// here, since the compositor is the source of truth for both.
pub(crate) fn create_shm_buffer(
    shm: &WlShm,
    qh: &QueueHandle<AppState>,
    width: i32,
    height: i32,
    stride: i32,
    format: WEnum<Format>,
) -> Result<(WlBuffer, memmap2::MmapMut), Box<dyn std::error::Error>> {
    let size = (stride as usize) * (height as usize);

    let fd = memfd_create("ricture-shm", MemfdFlags::CLOEXEC)?;
    ftruncate(&fd, size as u64)?;

    // SAFETY: `fd` was just created above; nothing else has it mapped or
    // is racing to resize it out from under us.
    let mmap = unsafe { memmap2::MmapMut::map_mut(&fd)? };

    let pool = shm.create_pool(fd.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(0, width, height, stride, format.into_result()?, qh, ());
    pool.destroy();

    Ok((buffer, mmap))
}

impl Dispatch<WlShm, ()> for AppState {
    fn event(
        _state: &mut Self,
        _shm: &WlShm,
        _event: wl_shm::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wl_shm::Event::Format — advertises a supported pixel format.
        // Not needed yet: we ask for a format ourselves once we know what
        // the screencopy frame supports.
    }
}

impl Dispatch<WlShmPool, ()> for AppState {
    fn event(
        _state: &mut Self,
        _pool: &WlShmPool,
        event: wl_shm_pool::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wl_shm_pool has no events; this is unreachable in practice, the
        // wildcard is only here because the generated enum is #[non_exhaustive].
        match event {
            _ => {}
        }
    }
}

impl Dispatch<WlBuffer, ()> for AppState {
    fn event(
        _state: &mut Self,
        _buffer: &WlBuffer,
        _event: wl_buffer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wl_buffer::Event::Release — compositor is done reading this
        // buffer and it's safe to reuse/free. We only ever capture once
        // and drop everything, so there's nothing to do here yet.
    }
}
