use crate::shell::{node, FLOATING_Z_INDEX, TILING_Z_INDEX};
use smithay::desktop::{Space, Window};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Resource;
use smithay::utils::{Logical, Point, Rectangle, Size};
use smithay::wayland::compositor;
use smithay::wayland::shell::xdg::{ToplevelSurface, XdgToplevelSurfaceRoleAttributes};
use std::cell::RefCell;
use std::fmt::Debug;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct WindowState {
    id: RefCell<u32>,
    floating: RefCell<bool>,
    configured: RefCell<bool>,
    initial_size: RefCell<Size<i32, Logical>>,
    size: RefCell<Size<i32, Logical>>,
    loc: RefCell<Point<i32, Logical>>,
}

impl WindowState {
    fn new() -> Self {
        WindowState {
            id: RefCell::new(node::id::next()),
            floating: RefCell::new(false),
            configured: RefCell::new(false),
            initial_size: RefCell::new(Default::default()),
            size: RefCell::new(Default::default()),
            loc: RefCell::new(Default::default()),
        }
    }

    pub fn id(&self) -> u32 {
        *self.id.borrow()
    }

    pub fn loc(&self) -> Point<i32, Logical> {
        *self.loc.borrow()
    }

    pub fn is_floating(&self) -> bool {
        *self.floating.borrow()
    }

    pub fn configured(&self) -> bool {
        *self.configured.borrow()
    }

    pub fn set_configured(&self) {
        self.configured.replace(true);
    }

    pub fn initial_size(&self) -> Size<i32, Logical> {
        *self.initial_size.borrow()
    }

    pub fn set_initial_geometry(&self, size: Size<i32, Logical>) {
        self.initial_size.replace(size);
    }

    pub fn toggle_floating(&self) {
        let current = *self.floating.borrow();
        self.floating.replace(!current);
    }
}

#[derive(Debug, Clone)]
pub struct WzmWindow {
    inner: Window,
}

#[derive(Debug)]
pub struct XdgTopLevelAttributes {
    pub app_id: Option<String>,
    pub title: Option<String>,
}

impl WzmWindow {
    pub fn update_floating(&self, zone: Rectangle<i32, Logical>) -> bool {
        let (size, location) = if self.get_state().configured() {
            let initial_size = self.get_state().initial_size();
            let size = initial_size;
            let location = self.center(zone.size);
            (Some(size), location)
        } else {
            (None, (0, 0).into())
        };

        self.update_loc_and_size(size, location)
    }

    pub fn set_fullscreen(&self, zone: Rectangle<i32, Logical>) {
        self.update_loc_and_size(Some(zone.size), zone.loc);
    }

    pub fn xdg_surface_attributes(&self) -> XdgTopLevelAttributes {
        compositor::with_states(self.wl_surface().unwrap(), |states| {
            let guard = states
                .data_map
                .get::<Mutex<XdgToplevelSurfaceRoleAttributes>>()
                .unwrap()
                .lock()
                .unwrap();

            XdgTopLevelAttributes {
                app_id: guard.app_id.clone(),
                title: guard.title.clone(),
            }
        })
    }

    pub fn get_state(&self) -> &WindowState {
        self.inner.user_data().get::<WindowState>().unwrap()
    }

    pub fn id(&self) -> u32 {
        *self
            .inner
            .user_data()
            .get::<WindowState>()
            .unwrap()
            .id
            .borrow()
    }

    pub fn wl_id(&self) -> u32 {
        self.inner
            .toplevel()
            .unwrap()
            .wl_surface()
            .id()
            .protocol_id()
    }

    pub fn location(&self) -> Point<i32, Logical> {
        *self.get_state().loc.borrow()
    }

    pub fn inner(&self) -> &Window {
        &self.inner
    }

    pub fn toplevel(&self) -> Option<&ToplevelSurface> {
        self.inner.toplevel()
    }

    pub fn wl_surface(&self) -> Option<&WlSurface> {
        self.inner.toplevel().map(|toplevel| toplevel.wl_surface())
    }

    pub fn map(&self, space: &mut Space<Window>, activate: bool) {
        if let Some(toplevel) = self.toplevel() {
            toplevel.with_pending_state(|state| {
                state.size = Some(self.size());
            });

            toplevel.send_configure();
        }

        space.map_element(self.inner.clone(), self.loc(), activate);
    }

    pub fn update_loc<P>(&self, location: P) -> bool
    where
        P: Into<Point<i32, Logical>> + Debug,
    {
        let state = self.get_state();
        let new_location = location.into();
        if *state.loc.borrow() != new_location {
            state.loc.replace(new_location);
            true
        } else {
            false
        }
    }

    pub fn update_loc_and_size<S, P>(&self, size: Option<S>, location: P) -> bool
    where
        S: Into<Size<i32, Logical>> + Debug,
        P: Into<Point<i32, Logical>> + Debug,
    {
        let state = self.get_state();
        let new_location = location.into();

        let loc_changed = if *state.loc.borrow() != new_location {
            state.loc.replace(new_location);
            true
        } else {
            false
        };

        let size_changed = if let Some(new_size) = size {
            let new_size = new_size.into();
            if *state.size.borrow() != new_size {
                state.size.replace(new_size);
                true
            } else {
                false
            }
        } else {
            false
        };

        loc_changed || size_changed
    }

    pub fn send_close(&self) {
        self.inner.toplevel().unwrap().send_close()
    }

    pub fn toggle_floating(&self) {
        self.get_state().toggle_floating();
    }

    pub fn is_floating(&self) -> bool {
        self.get_state().is_floating()
    }

    pub fn z_index(&self) -> u8 {
        if self.is_floating() {
            FLOATING_Z_INDEX
        } else {
            TILING_Z_INDEX
        }
    }

    pub fn center(&self, output_size: Size<i32, Logical>) -> Point<i32, Logical> {
        let center_y = output_size.h / 2;
        let center_x = output_size.w / 2;
        let window_geometry = self.inner.geometry();
        let window_center_y = window_geometry.size.h / 2;
        let window_center_x = window_geometry.size.w / 2;
        let x = center_x - window_center_x;
        let y = center_y - window_center_y;
        Point::from((x, y))
    }

    pub fn size(&self) -> Size<i32, Logical> {
        *self.get_state().size.borrow()
    }

    pub fn loc(&self) -> Point<i32, Logical> {
        *self.get_state().loc.borrow()
    }
}

impl From<ToplevelSurface> for WzmWindow {
    fn from(toplevel: ToplevelSurface) -> Self {
        let window = Window::new_wayland_window(toplevel);
        window.user_data().insert_if_missing(WindowState::new);
        WzmWindow { inner: window }
    }
}

impl From<Window> for WzmWindow {
    fn from(window: Window) -> Self {
        WzmWindow { inner: window }
    }
}

impl WzmWindow {
    pub fn from_x11_window(window: Window) -> WzmWindow {
        window.user_data().insert_if_missing(WindowState::new);
        WzmWindow { inner: window }
    }
}
