use nix::libc;
use std::borrow::Cow;
use std::cell::RefMut;
use std::io;
use std::os::unix::prelude::CommandExt;
use std::process::{Command, Stdio};

use smithay::desktop::Window;
use smithay::utils::{Point, Serial, SERIAL_COUNTER};
use smithay::wayland::seat::WaylandFocus;
use tracing::{debug, warn};

use wzm_config::action::Direction;
use wzm_config::keybinding::{Mode, ResizeDirection, ResizeType};

use crate::shell::{Orientation, Tree};
use crate::Wzm;

impl Wzm {
    pub fn set_layout_h(&mut self) {
        self.state.next_layout = Some(Orientation::Horizontal)
    }

    pub fn set_layout_v(&mut self) {
        self.state.next_layout = Some(Orientation::Vertical)
    }

    pub fn toggle_floating(&mut self) {
        //
    }

    pub fn toggle_fullscreen_window(&mut self) {
        todo!()
    }

    pub fn move_focus(&mut self, direction: Direction) {
        let ws = self.state.get_current_workspace();
        let mut ws = ws.borrow_mut();
        if let Some(window) = self.scan_window(direction, &ws) {
            if let Some(focus) = ws.get_node_for_data(&window) {
                ws.set_focus(focus);
                let serial = SERIAL_COUNTER.next_serial();
                self.toggle_window_focus(serial, &window);
            }
        }
    }

    fn toggle_window_focus(&mut self, serial: Serial, window: &Window) {
        let keyboard = self.state.seat.get_keyboard().unwrap();

        self.state.space.elements().for_each(|window| {
            if let Some(toplevel) = window.toplevel() {
                toplevel.send_configure();
            }
        });

        let location = self.state.space.element_bbox(window).unwrap().loc;

        self.state.space.map_element(window.clone(), location, true);

        keyboard.set_focus(self, window.wl_surface().map(Cow::into_owned), serial);

        window.set_activated(true);

        if let Some(toplevel) = window.toplevel() {
            toplevel.send_configure();
        }
    }

    fn scan_window(
        &mut self,
        direction: Direction,
        ws: &RefMut<'_, Tree<Window>>,
    ) -> Option<Window> {
        let mut window = None;
        if let Some(focus) = ws.get_focus() {
            let loc = self
                .state
                .space
                .element_location(&focus)
                .expect("window should have a location");

            let (mut x, mut y) = (loc.x, loc.y);
            let width = focus.geometry().size.w;
            let height = focus.geometry().size.h;

            // Move one pixel inside the window to avoid being out of bbox after converting to f64
            match direction {
                Direction::Right => {
                    x += width;
                    y += 1;
                }
                Direction::Down => y += height - 1,
                Direction::Left => y += 1,
                Direction::Up => x += 1,
            }

            let mut point = Point::from((x, y)).to_f64();
            while window.is_none() {
                if self.state.space.output_under(point).next().is_none() {
                    break;
                }

                direction.advance_point(&mut point);

                window = {
                    self.state
                        .space
                        .element_under(point)
                        .map(|(window, _)| window.clone())
                };
            }
        }
        window
    }

    pub fn close(&mut self) {
        let tree = self.state.get_current_workspace();
        let mut tree = tree.borrow_mut();
        if let Some(toplevel) = tree.get_focus().as_ref().and_then(|w| w.toplevel()) {
            toplevel.send_close();
        };

        tree.remove();

        if let Some(window) = tree.get_focus() {
            let handle = self
                .state
                .seat
                .get_keyboard()
                .expect("Should have a keyboard seat");

            let serial = SERIAL_COUNTER.next_serial();
            handle.set_focus(self, window.wl_surface().map(Cow::into_owned), serial);
        }
    }

    pub fn move_window(&mut self, direction: Direction) {
        let tree = self.state.get_current_workspace();
        let mut tree = tree.borrow_mut();
        if let Some(window) = self.scan_window(direction, &tree) {
            if let Some((tree_id, leaf_id)) = tree.get_node_for_data(&window) {
                tree.move_node(tree_id, leaf_id);
            }
        }
    }

    pub fn move_request_server(&mut self, serial: Serial, button_used: u32) {
        /*        debug!("Initiating move request from server");

                let pointer = self.seat.get_pointer().expect("seat had no pointer");
                let point = pointer.current_location();
                let Some((window, window_loc)) = self.space.element_under(point) else {
                    debug!("no window below cursor");
                    return;
                };

                // Return early if this is not a toplevel window
                if window.user_data().get::<WindowState>().is_none() {
                    debug!("not a toplevel window");
                    return;
                }

                let start_data = smithay::input::pointer::GrabStartData {
                    focus: pointer.current_focus().map(|focus| (focus, window_loc)),
                    button: button_used,
                    location: pointer.current_location(),
                };

                let grab = MoveSurfaceGrab {
                    start_data,
                    window: window.clone(),
                    initial_window_location: window_loc,
                };

                pointer.set_grab(self, grab, serial, Focus::Clear);
        */
    }

    pub fn toggle_resize(&mut self) {
        self.state.current_mode = match self.state.current_mode {
            Mode::Normal => Mode::Resize,
            Mode::Resize => Mode::Normal,
        };
    }

    pub fn toggle_layout(&mut self) {
        let ws = self.state.get_current_workspace();
        let mut ws = ws.borrow_mut();
        ws.toggle_layout();
    }

    pub fn resize(&mut self, kind: ResizeType, direction: ResizeDirection, amount: u32) {
        let ws = self.state.get_current_workspace();
        let mut ws = ws.borrow_mut();
        ws.resize(kind, direction, amount as i32);
    }
}

/// Spawns the command to run independently of the compositor.
pub fn spawn(cmd: String, env: Vec<(String, String)>) {
    debug!("spawning command: {cmd}, {env:?}");

    let mut process = Command::new(&cmd);
    process
        .envs(env)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    unsafe {
        // Double-fork to avoid having to waitpid the child.
        process.pre_exec(move || {
            match libc::fork() {
                -1 => return Err(io::Error::last_os_error()),
                0 => (),
                _ => libc::_exit(0),
            }

            Ok(())
        });
    }

    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(err) => {
            panic!("error spawning {cmd:?}: {err:?}");
        }
    };

    match child.wait() {
        Ok(status) => {
            if !status.success() {
                warn!("child did not exit successfully: {status:?}");
            }
        }
        Err(err) => {
            warn!("error waiting for child: {err:?}");
        }
    }
}
