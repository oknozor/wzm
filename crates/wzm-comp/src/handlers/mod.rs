use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Resource;
use smithay::wayland::output::OutputHandler;
use smithay::wayland::selection::data_device::{
    set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
    ServerDndGrabHandler,
};
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::xdg_foreign::{XdgForeignHandler, XdgForeignState};
use smithay::{delegate_data_device, delegate_output, delegate_seat, delegate_xdg_foreign};

use crate::Wzm;

mod activation;
mod compositor;
mod decoration;
mod dmabuf;
mod drm;
mod layer_shell;
mod xdg_shell;
//
// Wl Seat
//

impl SeatHandler for Wzm {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Wzm> {
        &mut self.state.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let dh = &self.state.display_handle;
        let client = focused.and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client);
    }
}

delegate_seat!(Wzm);

//
// Wl Data Device
//

impl SelectionHandler for Wzm {
    type SelectionUserData = ();
}

impl DataDeviceHandler for Wzm {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.state.data_device_state
    }
}

impl ClientDndGrabHandler for Wzm {}
impl ServerDndGrabHandler for Wzm {}

delegate_data_device!(Wzm);
impl OutputHandler for Wzm {}
delegate_output!(Wzm);

impl XdgForeignHandler for Wzm {
    fn xdg_foreign_state(&mut self) -> &mut XdgForeignState {
        &mut self.state.xdg_foreign_state
    }
}

delegate_xdg_foreign!(Wzm);
