use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use wzm_comp::backend::winit::Winit;
use wzm_comp::backend::Backend;
use wzm_comp::{CalloopData, Display, EventLoop, Wzm};
use wzm_config::WzmConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "wzm=debug,wzm_comp=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut event_loop: EventLoop<CalloopData> = EventLoop::try_new()?;
    let loop_signal = event_loop.get_signal();
    let event_loop_handle = event_loop.handle();
    let display: Display<Wzm> = Display::new()?;
    let winit = Winit::new(event_loop_handle.clone(), display.handle()).unwrap();
    let state = Wzm::new(event_loop_handle, display, winit.output());

    let mut data = CalloopData {
        wzm: state,
        config: WzmConfig::get().unwrap(),
        backend: Backend::Winit(winit),
        loop_signal,
    };

    data.backend.init(&mut data.wzm);
    data.start_compositor();

    event_loop
        .run(None, &mut data, |state| {
            let ws = state.wzm.get_current_workspace();
            let mut ws = ws.get_mut();
            if ws.needs_redraw {
                ws.update_layout(&state.wzm.space);
                ws.redraw(&mut state.wzm.space);
            }
        })
        .unwrap();

    Ok(())
}
