use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Arc;

use smithay::desktop::{PopupManager, Space, Window, WindowSurfaceType};
use smithay::input::{Seat, SeatState};
use smithay::output::Output;
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, LoopHandle, Mode, PostAction};
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Display, DisplayHandle};
use smithay::utils::{Logical, Point};
use smithay::wayland::compositor::{CompositorClientState, CompositorState};
use smithay::wayland::output::OutputManagerState;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::shell::wlr_layer::WlrLayerShellState;
use smithay::wayland::shell::xdg::decoration::XdgDecorationState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;
use smithay::wayland::socket::ListeningSocketSource;
use smithay::wayland::xdg_activation::XdgActivationState;

use wzm_config::WzmConfig;

use crate::shell::container::ContainerLayout;
use crate::shell::workspace::WorkspaceRef;
use crate::CalloopData;

pub struct Wzm {
    pub start_time: std::time::Instant,
    pub socket_name: OsString,
    pub display_handle: DisplayHandle,
    pub space: Space<Window>,
    // Smithay State
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Wzm>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,
    pub xdg_decoration_state: XdgDecorationState,
    pub xdg_activation_state: XdgActivationState,
    pub layer_shell_state: WlrLayerShellState,
    pub seat: Seat<Self>,
    // We should use this in calloopdata, not wazm
    pub config: WzmConfig,

    // Shell
    pub mod_pressed: bool,
    pub workspaces: HashMap<u8, WorkspaceRef>,
    pub current_workspace: u8,
    pub next_layout: Option<ContainerLayout>,
}

impl Wzm {
    pub fn new(
        event_loop: LoopHandle<CalloopData>,
        display: Display<Self>,
        output: &Output,
    ) -> Self {
        let start_time = std::time::Instant::now();

        let dh = display.handle();

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let popups = PopupManager::default();
        let xdg_decoration_state = XdgDecorationState::new::<Wzm>(&dh);
        let xdg_activation_state = XdgActivationState::new::<Wzm>(&dh);
        let layer_shell_state = WlrLayerShellState::new_with_filter::<Wzm, _>(&dh, |client| {
            !client.get_data::<ClientState>().unwrap().restricted
        });

        // A seat is a group of keyboards, pointer and touch devices.
        // A seat typically has a pointer and maintains a keyboard focus and a pointer focus.
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "winit");

        // Notify clients that we have a keyboard, for the sake of the example we assume that keyboard is always present.
        // You may want to track keyboard hot-plug in real compositor.
        seat.add_keyboard(Default::default(), 200, 25).unwrap();

        // Notify clients that we have a pointer (mouse)
        // Here we assume that there is always pointer plugged in
        seat.add_pointer();

        // A space represents a two-dimensional plane. Windows and Outputs can be mapped onto it.
        //
        // Windows get a position and stacking order through mapping.
        // Outputs become views of a part of the Space and can be rendered via Space::render_output.
        let mut space = Space::default();
        let socket_name = Self::init_wayland_listener(display, event_loop);
        space.map_output(output, (0, 0));

        Self {
            start_time,
            display_handle: dh,
            space,
            socket_name,
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            popups,
            xdg_decoration_state,
            xdg_activation_state,
            layer_shell_state,
            seat,
            config: WzmConfig::get().expect("failed to get config"),
            mod_pressed: false,
            workspaces: Default::default(),
            current_workspace: 0,
            next_layout: None,
        }
    }

    fn init_wayland_listener(
        display: Display<Wzm>,
        event_loop: LoopHandle<CalloopData>,
    ) -> OsString {
        // Creates a new listening socket, automatically choosing the next available `wayland` socket name.
        let listening_socket = ListeningSocketSource::new_auto().unwrap();

        // Get the name of the listening socket.
        // Clients will connect to this socket.
        let socket_name = listening_socket.socket_name().to_os_string();

        event_loop
            .insert_source(listening_socket, move |client_stream, _, state| {
                // Inside the callback, you should insert the client into the display.
                //
                // You may also associate some data with the client when inserting the client.
                state
                    .wzm
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        // You also need to add the display itself to the event loop, so that client events will be processed by wayland-server.
        event_loop
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: we don't drop the display
                    unsafe {
                        display.get_mut().dispatch_clients(&mut state.wzm).unwrap();
                    }
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        socket_name
    }

    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, p + location))
            })
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
    pub restricted: bool,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
