use std::time::Duration;

use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::ImportEgl;
use smithay::backend::winit::{self, WinitEvent};
use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
use smithay::reexports::calloop::EventLoop;
use smithay::utils::{Rectangle, Transform};
use tracing::info;

use crate::shell::workspace::WorkspaceRef;
use crate::{CalloopData, DisplayHandle, Wzm};

pub fn init_winit(
    event_loop: &mut EventLoop<CalloopData>,
    data: &mut Wzm,
    display_handle: DisplayHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = data;

    let (mut backend, winit) = winit::init::<GlesRenderer>()?;

    if backend.renderer().bind_wl_display(&display_handle).is_ok() {
        info!("EGL hardware-acceleration enabled");
    };

    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000,
    };

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Smithay".into(),
            model: "Winit".into(),
        },
    );
    let _global = output.create_global::<Wzm>(&display_handle);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);
    state.space.map_output(&output, (0, 0));
    if let Some(output) = state.space.outputs().next() {
        state.workspaces.insert(
            0,
            WorkspaceRef::new(output.clone(), &state.space, state.config.gaps as i32),
        );
    } else {
        panic!("Failed to create Workspace 0 on default Output");
    }

    let mut damage_tracker = OutputDamageTracker::from_output(&output);
    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    event_loop
        .handle()
        .insert_source(winit, move |event, _, data| {
            let display = &mut data.display_handle;
            let ws = data.state.get_current_workspace();
            let mut ws = ws.get_mut();

            let state = &mut data.state;
            let space = &mut state.space;

            if ws.needs_redraw {
                ws.update_layout(space);
                ws.redraw(space);
                ws.update_borders();
            }

            match event {
                WinitEvent::Resized { size, .. } => {
                    output.change_current_state(
                        Some(Mode {
                            size,
                            refresh: 60_000,
                        }),
                        None,
                        None,
                        None,
                    );
                }
                WinitEvent::Input(event) => state.process_input_event(event),
                WinitEvent::Redraw => {
                    let size = backend.window_size();
                    let damage = Rectangle::from_loc_and_size((0, 0), size);

                    backend.bind().unwrap();
                    smithay::desktop::space::render_output::<
                        _,
                        WaylandSurfaceRenderElement<GlesRenderer>,
                        _,
                        _,
                    >(
                        &output,
                        backend.renderer(),
                        1.0,
                        0,
                        [&state.space],
                        &[],
                        &mut damage_tracker,
                        [0.1, 0.1, 0.1, 1.0],
                    )
                    .unwrap();
                    backend.submit(Some(&[damage])).unwrap();

                    state.space.elements().for_each(|window| {
                        window.send_frame(
                            &output,
                            state.start_time.elapsed(),
                            Some(Duration::ZERO),
                            |_, _| Some(output.clone()),
                        )
                    });

                    state.space.refresh();
                    state.popups.cleanup();
                    let _ = display.flush_clients();

                    // Ask for redraw to schedule new frame.
                    backend.window().request_redraw();
                }
                WinitEvent::CloseRequested => {
                    state.loop_signal.stop();
                }
                _ => (),
            };
        })?;

    Ok(())
}
