use crate::state;
use crate::state::AppState;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    Event, ZwlrScreencopyFrameV1,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::{
    self, ZwlrScreencopyManagerV1,
};

impl Dispatch<ZwlrScreencopyManagerV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _manager: &ZwlrScreencopyManagerV1,
        event: zwlr_screencopy_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            _ => {}
        }
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                state.buffer = Some(state::Buffer {
                    format: format.clone(),
                    width: width as i32,
                    height: height as i32,
                    stride: stride as i32,
                })
            }
            Event::BufferDone => {
                let state_buf = state.buffer.as_ref().unwrap();
                let buf = crate::wayland::shm::create_shm_buffer(
                    state.shm.as_ref().unwrap(),
                    qhandle,
                    state_buf.width,
                    state_buf.height,
                    state_buf.stride,
                    state_buf.format,
                )
                .unwrap();
                proxy.copy(&buf.0);
                state.wl_buffer = Some(buf.0);
                state.mmap_mut = Some(buf.1);
            }
            Event::Ready { .. } => {
                state.capture_done = true;
            }
            _ => {}
        }
    }
}

pub fn capture_first_output() -> Result<crate::state::Screenshot, Box<dyn std::error::Error>> {
    let (_conn, mut eq, mut state) = crate::wayland::connection::connect()?;
    let qh = eq.handle();
    let _frame =
        state
            .screencopy_manager
            .as_ref()
            .unwrap()
            .capture_output(0, &state.outputs[0], &qh, ());

    while !state.capture_done {
        eq.blocking_dispatch(&mut state)?;
    }

    let buffer = state.buffer.as_ref().unwrap();
    let mmap = state.mmap_mut.as_ref().unwrap();
    crate::convert::to_rgba(buffer, mmap)
}
