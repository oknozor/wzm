use std::collections::HashMap;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::gbm::GbmDevice;
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmNode};
use smithay::backend::renderer::{DebugFlags, ImportDma};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::multigpu::gbm::GbmGlesBackend;
use smithay::backend::renderer::multigpu::{GpuManager, MultiRenderer};
use smithay::backend::session::libseat::LibSeatSession;
use smithay::delegate_dmabuf;
use smithay::reexports::calloop::RegistrationToken;
use smithay::reexports::drm::control::{connector, crtc};
use smithay::reexports::input::ffi::udev;
use smithay::wayland::compositor::SurfaceData;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier};
use smithay::wayland::drm_lease::{DrmLease, DrmLeaseState};
use smithay_drm_extras::drm_scanner::DrmScanner;
use crate::{Wzm, DisplayHandle, State};
use crate::backend::Backend;

pub struct Udev {
    pub session: LibSeatSession,
    dh: DisplayHandle,
    dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    pub(crate) primary_gpu: DrmNode,
    pub(crate) gpus: GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    backends: HashMap<DrmNode, BackendData>,
    // pointer_images: Vec<(xcursor::parser::Image, MemoryRenderBuffer)>,
    // pointer_element: PointerElement,
    // TODO: pointer_image: crate::cursor::Cursor,
    debug_flags: DebugFlags,
    keyboards: Vec<smithay::reexports::input::Device>,
}

struct BackendData {
    surfaces: HashMap<crtc::Handle, SurfaceData>,
    non_desktop_connectors: Vec<(connector::Handle, crtc::Handle)>,
    leasing_global: Option<DrmLeaseState>,
    active_leases: Vec<DrmLease>,
    gbm: GbmDevice<DrmDeviceFd>,
    drm: DrmDevice,
    drm_scanner: DrmScanner,
    render_node: DrmNode,
    registration_token: RegistrationToken,
}

type UdevRenderer<'a> = MultiRenderer<
    'a,
    'a,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
>;

#[derive(Debug, PartialEq)]
struct UdevOutputId {
    device_id: DrmNode,
    crtc: crtc::Handle,
}

impl DmabufHandler for Wzm {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        match &mut self.backend {
            Backend::Winit(winit) => &mut winit.dmabuf_state.0,
            Backend::Udev(udev) => {
                &mut udev.dmabuf_state.as_mut().unwrap().0
            }
        }
    }

    fn dmabuf_imported(&mut self, global: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier) {
        match &mut self.backend {
            Backend::Winit(winit) => {
                if winit
                    .renderer()
                    .import_dmabuf(&dmabuf, None)
                    .is_ok()
                {
                    let _ = notifier.successful::<Wzm>();
                } else {
                    notifier.failed();
                }
            }
            Backend::Udev(udev) => {
                if udev.gpus.single_renderer(&udev.primary_gpu)
                    .and_then(|mut renderer| renderer.import_dmabuf(&dmabuf, None))
                    .is_ok() {
                    dmabuf.set_node(*&udev.primary_gpu);
                    let _ = notifier.successful::<Wzm>();
                } else {
                    notifier.failed()
                }
            }
        };
    }
}

delegate_dmabuf!(Wzm);
