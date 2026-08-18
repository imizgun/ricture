use std::fs::File;
use std::io::{ErrorKind, Write};
use std::os::fd::{AsRawFd};

use super::state::ClipboardState;
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::{
    self, ZwlrDataControlDeviceV1,
};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::{
    self, ZwlrDataControlSourceV1,
};

// Receiving side of the protocol (data_offer / selection / finished)
impl Dispatch<ZwlrDataControlDeviceV1, ()> for ClipboardState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlDeviceV1,
        _event: zwlr_data_control_device_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
    event_created_child!(ClipboardState, ZwlrDataControlDeviceV1, [
        0 => (ZwlrDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for ClipboardState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlOfferV1,
        _event: <ZwlrDataControlOfferV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

const POLLOUT: i16 = 0x0004;

unsafe extern "C" {
    fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
}

fn wait_writable(fd: i32) {
    let mut pfd = PollFd { fd, events: POLLOUT, revents: 0 };
    // SAFETY: pfd is a valid pointer to one pollfd, alive for the call.
    unsafe { poll(&mut pfd, 1, -1) };
}

impl Dispatch<ZwlrDataControlSourceV1, ()> for ClipboardState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrDataControlSourceV1,
        event: zwlr_data_control_source_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_source_v1::Event::Send { fd, .. } => {
                let mut file = File::from(fd);

                let to_write = state.payload.len();
                let mut written = 0;

                while written != to_write {
                    match file.write(&state.payload[written..]) {
                        Ok(b) => {
                            written += b;
                        },
                        Err(err) => {
                            match err.kind() {
                                ErrorKind::WouldBlock => {
                                    wait_writable(file.as_raw_fd());
                                },
                                _ => {
                                    eprintln!("error when writing fd: {}", err);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            zwlr_data_control_source_v1::Event::Cancelled => {
                proxy.destroy();
                state.done = true;
            }
            _ => {}
        }
    }
}
