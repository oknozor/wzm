use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::reexports::wayland_server::protocol::wl_buffer;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Client;
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler, CompositorState,
};
use smithay::wayland::shm::{ShmHandler, ShmState};
use smithay::{delegate_compositor, delegate_shm};

use crate::grabs::resize_grab;
use crate::state::ClientState;
use crate::{Wzm, State};

use super::xdg_shell;

impl CompositorHandler for Wzm {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.state.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .state
                .space
                .elements()
                .find(|w| w.toplevel().unwrap().wl_surface() == &root)
            {
                window.on_commit();
            }
        };

        self.state.layer_shell_handle_commit(surface);
        xdg_shell::handle_commit(&mut self.state.popups, &self.state.space, surface);
        resize_grab::handle_commit(&mut self.state.space, surface);
    }

    fn destroyed(&mut self, _: &WlSurface) {
        let ws = self.state.get_current_workspace();
        // ws.update_layout();
    }
}

impl BufferHandler for Wzm {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for Wzm {
    fn shm_state(&self) -> &ShmState {
        &self.state.shm_state
    }
}

delegate_compositor!(Wzm);
delegate_shm!(Wzm);
