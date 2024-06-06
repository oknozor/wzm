use crate::backend::Backend;
use crate::Wzm;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::ImportDma;
use smithay::delegate_dmabuf;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier};

impl DmabufHandler for Wzm {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        match &mut self.backend {
            Backend::Winit(winit) => &mut winit.dmabuf_state.0,
            Backend::Udev(udev) => &mut udev.dmabuf_state.as_mut().unwrap().0,
        }
    }

    fn dmabuf_imported(&mut self, global: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier) {
        match &mut self.backend {
            Backend::Winit(winit) => {
                if winit.renderer().import_dmabuf(&dmabuf, None).is_ok() {
                    let _ = notifier.successful::<Wzm>();
                } else {
                    notifier.failed();
                }
            }
            Backend::Udev(udev) => {
                if udev
                    .gpus
                    .single_renderer(&udev.primary_gpu)
                    .and_then(|mut renderer| renderer.import_dmabuf(&dmabuf, None))
                    .is_ok()
                {
                    dmabuf.set_node(udev.primary_gpu);
                    let _ = notifier.successful::<Wzm>();
                } else {
                    notifier.failed()
                }
            }
        };
    }
}

delegate_dmabuf!(Wzm);
