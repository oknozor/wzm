use crate::DisplayHandle;
use smithay::backend::allocator::gbm::GbmDevice;
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmNode};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::multigpu::gbm::GbmGlesBackend;
use smithay::backend::renderer::multigpu::{GpuManager, MultiRenderer};
use smithay::backend::renderer::DebugFlags;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::reexports::calloop::RegistrationToken;
use smithay::reexports::drm::control::{connector, crtc};
use smithay::wayland::compositor::SurfaceData;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufState};
use smithay::wayland::drm_lease::{DrmLease, DrmLeaseState};
use smithay_drm_extras::drm_scanner::DrmScanner;
use std::collections::HashMap;

pub struct Udev {
    pub session: LibSeatSession,
    dh: DisplayHandle,
    pub(crate) dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    pub(crate) primary_gpu: DrmNode,
    pub(crate) gpus: GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    pub(crate) backends: HashMap<DrmNode, BackendData>,
    // pointer_images: Vec<(xcursor::parser::Image, MemoryRenderBuffer)>,
    // pointer_element: PointerElement,
    // TODO: pointer_image: crate::cursor::Cursor,
    debug_flags: DebugFlags,
    keyboards: Vec<smithay::reexports::input::Device>,
}

pub struct BackendData {
    surfaces: HashMap<crtc::Handle, SurfaceData>,
    pub(crate) non_desktop_connectors: Vec<(connector::Handle, crtc::Handle)>,
    pub(crate) leasing_global: Option<DrmLeaseState>,
    pub(crate) active_leases: Vec<DrmLease>,
    gbm: GbmDevice<DrmDeviceFd>,
    pub(crate) drm: DrmDevice,
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
