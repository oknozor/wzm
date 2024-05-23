use crate::shell::container::ContainerLayout;
use crate::Wzm;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1;
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::XdgToplevelSurfaceData;

impl Wzm {
    pub fn set_layout_h(&mut self) {
        self.next_layout = Some(ContainerLayout::Horizontal)
    }

    pub fn set_layout_v(&mut self) {
        self.next_layout = Some(ContainerLayout::Vertical)
    }

    pub fn toggle_decoration(&mut self) {
        for element in self.space.elements() {
            #[allow(irrefutable_let_patterns)]
            if let Some(toplevel) = element.toplevel() {
                let mode_changed = toplevel.with_pending_state(|state| {
                    if let Some(current_mode) = state.decoration_mode {
                        let new_mode =
                            if current_mode == zxdg_toplevel_decoration_v1::Mode::ClientSide {
                                zxdg_toplevel_decoration_v1::Mode::ServerSide
                            } else {
                                zxdg_toplevel_decoration_v1::Mode::ClientSide
                            };
                        state.decoration_mode = Some(new_mode);
                        true
                    } else {
                        false
                    }
                });

                let initial_configure_sent = with_states(toplevel.wl_surface(), |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .initial_configure_sent
                });

                if mode_changed && initial_configure_sent {
                    toplevel.send_pending_configure();
                }
            }
        }
    }
}
