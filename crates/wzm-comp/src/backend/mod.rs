use crate::backend::winit::Winit;
use crate::Wzm;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::ImportEgl;
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Transform;
use tracing::warn;

pub mod winit;

// Thank you niri
#[derive(PartialEq, Eq)]
pub enum RenderResult {
    /// The frame was submitted to the backend for presentation.
    Submitted,
    /// Rendering succeeded, but there was no damage.
    NoDamage,
    /// The frame was not rendered and submitted, due to an error or otherwise.
    Skipped,
}

pub enum Backend {
    Winit(Winit),
    Udev(()),
}

impl Backend {
    pub fn init(&mut self, wzm: &mut Wzm) {
        match self {
            Backend::Winit(winit) => {
                let renderer = winit.renderer();
                if let Err(err) = renderer.bind_wl_display(&wzm.display_handle) {
                    warn!("error binding renderer wl_display: {err}");
                }

                let output = &winit.output();
                output.change_current_state(None, Some(Transform::Flipped180), None, None);
                wzm.space.map_output(output, (0, 0));
            }
            Backend::Udev(_) => todo!(),
        }
    }

    pub fn seat_name(&self) -> String {
        todo!()
    }

    pub fn with_primary_renderer<T>(
        &mut self,
        _f: impl FnOnce(&mut GlesRenderer) -> T,
    ) -> Option<T> {
        todo!()
    }

    pub fn render(&mut self, wzm: &mut Wzm) {
        match self {
            Backend::Winit(winit) => winit.render(wzm),
            Backend::Udev(_) => todo!(),
        };
    }

    pub fn change_vt(&mut self, _vt: i32) {
        todo!()
    }

    pub fn suspend(&mut self) {
        todo!()
    }

    pub fn import_dmabuf(&mut self, _dmabuf: &Dmabuf) -> bool {
        todo!()
    }

    pub fn early_import(&mut self, _surface: &WlSurface) {
        todo!()
    }

    pub fn set_monitors_active(&mut self, _active: bool) {
        todo!()
    }

    pub fn on_output_config_changed(&mut self, _wzm: &mut Wzm) {
        todo!()
    }

    pub fn get_output(&self) -> &Output {
        match self {
            Backend::Winit(winit) => winit.output(),
            Backend::Udev(_) => {
                todo!()
            }
        }
    }
}
