use smithay::delegate_xdg_decoration;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1;
use smithay::wayland::shell::xdg::decoration::{XdgDecorationHandler};
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use tracing::debug;
use crate::Wzm;

impl XdgDecorationHandler for Wzm {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ServerSide);
        });
    }

    fn request_mode(&mut self, _toplevel: ToplevelSurface, _mode: Mode) {
        //TODO
    }

    fn unset_mode(&mut self, _toplevel: ToplevelSurface) {
        // TODO
    }
}

delegate_xdg_decoration!(Wzm);
