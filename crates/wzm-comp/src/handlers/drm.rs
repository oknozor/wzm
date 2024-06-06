use crate::backend::Backend;
use crate::Wzm;
use smithay::backend::drm::DrmNode;
use smithay::delegate_drm_lease;
use smithay::wayland::drm_lease::{
    DrmLease, DrmLeaseBuilder, DrmLeaseHandler, DrmLeaseRequest, DrmLeaseState, LeaseRejected,
};

impl DrmLeaseHandler for Wzm {
    fn drm_lease_state(&mut self, node: DrmNode) -> &mut DrmLeaseState {
        match &mut self.backend {
            Backend::Winit(_winit) => {
                todo!("Do we need drm lease for winit ?");
            }
            Backend::Udev(udev) => udev
                .backends
                .get_mut(&node)
                .unwrap()
                .leasing_global
                .as_mut()
                .unwrap(),
        }
    }

    fn lease_request(
        &mut self,
        node: DrmNode,
        request: DrmLeaseRequest,
    ) -> Result<DrmLeaseBuilder, LeaseRejected> {
        match &mut self.backend {
            Backend::Winit(_winit) => {
                todo!("Do we need drm lease for winit ?");
            }
            Backend::Udev(udev) => {
                let backend = udev.backends.get(&node).ok_or(LeaseRejected::default())?;

                let mut builder = DrmLeaseBuilder::new(&backend.drm);
                for conn in request.connectors {
                    if let Some((_, crtc)) = backend
                        .non_desktop_connectors
                        .iter()
                        .find(|(handle, _)| *handle == conn)
                    {
                        builder.add_connector(conn);
                        builder.add_crtc(*crtc);
                        let planes = backend
                            .drm
                            .planes(crtc)
                            .map_err(LeaseRejected::with_cause)?;
                        builder.add_plane(planes.primary.handle);
                        if let Some(cursor) = planes.cursor {
                            builder.add_plane(cursor.handle);
                        }
                    } else {
                        tracing::warn!(
                            ?conn,
                            "Lease requested for desktop connector, denying request"
                        );
                        return Err(LeaseRejected::default());
                    }
                }

                Ok(builder)
            }
        }
    }

    fn new_active_lease(&mut self, node: DrmNode, lease: DrmLease) {
        match &mut self.backend {
            Backend::Winit(_) => {
                todo!("Do we need drm lease for winit ?");
            }
            Backend::Udev(udev) => {
                let backend = udev.backends.get_mut(&node).unwrap();
                backend.active_leases.push(lease);
            }
        }
    }

    fn lease_destroyed(&mut self, node: DrmNode, lease: u32) {
        match &mut self.backend {
            Backend::Winit(_) => {
                todo!("Do we need drm lease for winit ?");
            }
            Backend::Udev(udev) => {
                let backend = udev.backends.get_mut(&node).unwrap();
                backend.active_leases.retain(|l| l.id() != lease);
            }
        }
    }
}

delegate_drm_lease!(Wzm);
