use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use wzm_comp::{winit, CalloopData, Display, EventLoop, Wzm};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "wzm=debug,wzm_comp=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut event_loop: EventLoop<CalloopData> = EventLoop::try_new()?;

    let display: Display<Wzm> = Display::new()?;
    let display_handle = display.handle();
    let state = Wzm::new(&mut event_loop, display);

    let mut data = CalloopData {
        state,
        display_handle: display_handle.clone(),
    };

    winit::init_winit(&mut event_loop, &mut data.state, display_handle)?;

    let mut args = std::env::args().skip(1);
    let flag = args.next();
    let arg = args.next();

    match (flag.as_deref(), arg) {
        (Some("-c") | Some("--command"), Some(command)) => {
            std::process::Command::new(command).spawn().ok();
        }
        _ => {
            std::process::Command::new("weston-terminal").spawn().ok();
        }
    }

    event_loop.run(None, &mut data, move |_| {
        // Smallvil is running
    })?;

    Ok(())
}
