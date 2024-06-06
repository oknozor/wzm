use std::time::Duration;
use smithay::backend::egl::EGLDevice;

use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::{ImportDma, ImportEgl};
use smithay::backend::winit;
use smithay::backend::winit::{WinitEvent, WinitGraphicsBackend};
use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::winit::dpi::LogicalSize;
use smithay::reexports::winit::window::WindowBuilder;
use smithay::utils::Rectangle;
use smithay::wayland::dmabuf::{DmabufFeedback, DmabufFeedbackBuilder, DmabufGlobal, DmabufState};
use tracing::{info, warn};

use crate::decoration::{BorderShader, CustomRenderElements};
use crate::{Wzm, DisplayHandle, State};

pub struct Winit {
    output: Output,
    backend: WinitGraphicsBackend<GlesRenderer>,
    damage_tracker: OutputDamageTracker,
    pub dmabuf_state: (DmabufState, DmabufGlobal, Option<DmabufFeedback>),
}

impl Winit {
    pub fn new(
        event_loop: LoopHandle<Wzm>,
        display_handle: DisplayHandle,
    ) -> Result<Self, winit::Error> {
        let builder = WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1920.0, 1080.0))
            .with_resizable(false)
            .with_title("wzm");

        let (mut backend, winit) = winit::init_from_builder::<GlesRenderer>(builder)?;
        BorderShader::init(backend.renderer());

        if backend.renderer().bind_wl_display(&display_handle).is_ok() {
            info!("EGL hardware-acceleration enabled");
        }

        let output = Output::new(
            "winit".to_string(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "Smithay".into(),
                model: "Winit".into(),
            },
        );

        let mode = Mode {
            size: backend.window_size(),
            refresh: 60_000,
        };

        let _global = output.create_global::<Wzm>(&display_handle);
        output.change_current_state(Some(mode), None, None, None);
        output.set_preferred(mode);

        let dmabuf_state = Self::init_dmabuf_state(&display_handle, &mut backend);

        let damage_tracker = OutputDamageTracker::from_output(&output);

        event_loop
            .insert_source(winit, move |event, _, state| match event {
                WinitEvent::Resized { .. } => {
                    let data = &mut state.backend;
                    let output = data.get_output();
                    output.change_current_state(Some(mode), None, None, None);
                    output.set_preferred(mode);
                }
                WinitEvent::Input(event) => state.process_input_event(event),
                WinitEvent::Focus(_) => (),
                WinitEvent::Redraw => state.backend.render(&mut state.state),
                WinitEvent::CloseRequested => state.loop_signal.stop(),
            })
            .unwrap();

        Ok(Self {
            output,
            backend,
            damage_tracker,
            dmabuf_state,
        })
    }

    fn init_dmabuf_state(display_handle: &DisplayHandle, mut backend: &mut WinitGraphicsBackend<GlesRenderer>) -> (DmabufState, DmabufGlobal, Option<DmabufFeedback>) {
        let render_node = EGLDevice::device_for_display(backend.renderer().egl_context().display())
            .and_then(|device| device.try_get_render_node());

        let dmabuf_default_feedback = match render_node {
            Ok(Some(node)) => {
                let dmabuf_formats = backend.renderer().dmabuf_formats().collect::<Vec<_>>();
                let dmabuf_default_feedback = DmabufFeedbackBuilder::new(node.dev_id(), dmabuf_formats)
                    .build()
                    .unwrap();
                Some(dmabuf_default_feedback)
            }
            Ok(None) => {
                warn!("failed to query render node, dmabuf will use v3");
                None
            }
            Err(err) => {
                warn!(?err, "failed to egl device for display, dmabuf will use v3");
                None
            }
        };

        // if we failed to build dmabuf feedback we fall back to dmabuf v3
        // Note: egl on Mesa requires either v4 or wl_drm (initialized with bind_wl_display)
        let dmabuf_state = if let Some(default_feedback) = dmabuf_default_feedback {
            let mut dmabuf_state = DmabufState::new();
            let dmabuf_global = dmabuf_state.create_global_with_default_feedback::<Wzm>(
                display_handle,
                &default_feedback,
            );
            (dmabuf_state, dmabuf_global, Some(default_feedback))
        } else {
            let dmabuf_formats = backend.renderer().dmabuf_formats().collect::<Vec<_>>();
            let mut dmabuf_state = DmabufState::new();
            let dmabuf_global =
                dmabuf_state.create_global::<Wzm>(display_handle, dmabuf_formats);
            (dmabuf_state, dmabuf_global, None)
        };
        dmabuf_state
    }

    pub fn render(&mut self, wzm: &mut State) {
        let size = self.backend.window_size();
        let damage = Rectangle::from_loc_and_size((0, 0), size);

        self.backend.bind().unwrap();

        smithay::desktop::space::render_output::<_, CustomRenderElements<GlesRenderer>, _, _>(
            &self.output,
            self.backend.renderer(),
            1.0,
            0,
            [&wzm.space],
            &[],
            &mut self.damage_tracker,
            [0.1, 0.1, 0.1, 1.0],
        )
        .unwrap();

        self.backend.submit(Some(&[damage])).unwrap();

        wzm.space.elements().for_each(|window| {
            window.send_frame(
                &self.output,
                wzm.start_time.elapsed(),
                Some(Duration::ZERO),
                |_, _| Some(self.output.clone()),
            )
        });

        wzm.space.refresh();
        wzm.popups.cleanup();
        let _ = wzm.display_handle.flush_clients();

        // Ask for redraw to schedule new frame.
        self.backend.window().request_redraw();
    }

    pub fn output(&self) -> &Output {
        &self.output
    }

    pub fn renderer(&mut self) -> &mut GlesRenderer {
        self.backend.renderer()
    }
}
