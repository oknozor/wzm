use smithay::desktop::Window;
use smithay::utils::{Point, Serial, SERIAL_COUNTER};
use tracing::debug;

use wzm_config::action::Direction;

use crate::shell::container::{ContainerLayout, ContainerState};
use crate::shell::node::Node;
use crate::shell::windows::{WindowState, WindowWrap};
use crate::Wzm;

impl Wzm {
    pub fn set_layout_h(&mut self) {
        self.next_layout = Some(ContainerLayout::Horizontal)
    }

    pub fn set_layout_v(&mut self) {
        self.next_layout = Some(ContainerLayout::Vertical)
    }

    pub fn toggle_floating(&mut self) {
        let ws = self.get_current_workspace();
        let mut ws = ws.get_mut();
        let focus = ws.get_focus();

        if let Some(window) = focus.1 {
            window.toggle_floating();
            let output_geometry = self.space.output_geometry(&ws.output).unwrap();
            let redraw = focus.0.get_mut().update_layout(output_geometry);
            ws.needs_redraw = redraw;
        }
    }

    pub fn toggle_fullscreen_window(&mut self) {
        let ws = self.get_current_workspace();
        let mut ws = ws.get_mut();

        if ws.fullscreen_layer.is_some() {
            ws.fullscreen_layer = None;
            ws.update_layout(&self.space);
        } else {
            let (_c, window) = ws.get_focus();
            if let Some(window) = window {
                ws.fullscreen_layer = Some(Node::Window(window));
            }
        }

        ws.needs_redraw = true;
    }

    pub fn toggle_fullscreen_container(&mut self) {
        let ws = self.get_current_workspace();
        let mut ws = ws.get_mut();
        if ws.fullscreen_layer.is_some() {
            ws.reset_gaps(&self.space);
            ws.fullscreen_layer = None;
            ws.update_layout(&self.space);
        } else {
            let (container, _) = ws.get_focus();
            let output_geometry = self.space.output_geometry(&ws.output).unwrap();
            container
                .get_mut()
                .set_fullscreen_loc_and_size(output_geometry);
            ws.fullscreen_layer = Some(Node::Container(container));
        }

        ws.needs_redraw = true
    }

    pub fn move_focus(&mut self, direction: Direction) {
        let window = self.scan_window(direction);

        if let Some(window) = window {
            let serial = SERIAL_COUNTER.next_serial();
            let id = window.user_data().get::<WindowState>().unwrap().id();
            let ws = self.get_current_workspace();
            let mut ws = ws.get_mut();
            let container = ws.root().container_having_window(id).unwrap();
            let window = WindowWrap::from(window);
            ws.set_container_and_window_focus(&container, &window);
            self.toggle_window_focus(serial, window.inner());
        }
    }

    fn toggle_window_focus(&mut self, serial: Serial, window: &Window) {
        let keyboard = self.seat.get_keyboard().unwrap();

        self.space.elements().for_each(|window| {
            if let Some(toplevel) = window.toplevel() {
                toplevel.send_configure();
            }
        });

        let window = WindowWrap::from(window.clone());
        let location = self.space.element_bbox(window.inner()).unwrap().loc;

        self.space
            .map_element(window.inner().clone(), location, true);

        keyboard.set_focus(self, window.wl_surface().cloned(), serial);

        let window = window.inner();

        window.set_activated(true);

        if let Some(toplevel) = window.toplevel() {
            toplevel.send_configure();
        }
    }

    fn scan_window(&mut self, direction: Direction) -> Option<Window> {
        let ws = self.get_current_workspace();
        let ws = ws.get();
        let focus = ws.get_focus();
        let mut window = None;
        if let Some(window_ref) = focus.1 {
            let loc = self
                .space
                .element_location(window_ref.inner())
                .expect("window should have a location");

            let (mut x, mut y) = (loc.x, loc.y);
            let width = window_ref.inner().geometry().size.w;
            let height = window_ref.inner().geometry().size.h;

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
                if self.space.output_under(point).next().is_none() {
                    break;
                }

                direction.advance_point(&mut point);

                window = {
                    self.space
                        .element_under(point)
                        .map(|(window, _)| window.clone())
                };
            }
        }
        window
    }

    pub fn close(&mut self) {
        let state = {
            let container = self.get_current_workspace().get_mut().get_focus().0;
            let mut container = container.get_mut();
            debug!("Closing window in container: {}", container.id);
            container.close_window();
            container.state()
        };

        match state {
            ContainerState::Empty => {
                debug!("Closing empty container");
                let ws = self.get_current_workspace();
                let mut ws = ws.get_mut();
                ws.pop_container();
                if let Some(window) = ws.get_focus().1 {
                    self.toggle_window_focus(SERIAL_COUNTER.next_serial(), window.inner());
                }
            }
            ContainerState::HasContainersOnly => {
                debug!("Draining window from container");
                {
                    let container = {
                        let ws = self.get_current_workspace();
                        let ws = &ws.get_mut();
                        ws.get_focus().0
                    };

                    let children: Option<Vec<(u32, Node)>> = {
                        let mut container = container.get_mut();
                        if container.parent.is_some() {
                            Some(container.nodes.drain_all())
                        } else {
                            None
                        }
                    };

                    let mut container = container.get_mut();
                    let id = container.id;

                    if let (Some(parent), Some(children)) = (&mut container.parent, children) {
                        let mut parent = parent.get_mut();
                        parent.nodes.remove(&id);
                        parent.nodes.extend(children);
                    }
                }

                let ws = self.get_current_workspace();
                let ws = ws.get();

                if let Some(window) = ws.get_focus().1 {
                    self.toggle_window_focus(SERIAL_COUNTER.next_serial(), window.inner());
                }
            }
            ContainerState::HasWindows => {
                let ws = self.get_current_workspace();
                let ws = ws.get();

                if let Some(window) = ws.get_focus().1 {
                    self.toggle_window_focus(SERIAL_COUNTER.next_serial(), window.inner());
                }
                debug!("Cannot remove non empty container");
            }
        };

        // Reset focus
        let workspace = self.get_current_workspace();
        let mut workspace = workspace.get_mut();

        {
            if let Some(window) = workspace.get_focus().1 {
                let handle = self
                    .seat
                    .get_keyboard()
                    .expect("Should have a keyboard seat");

                let serial = SERIAL_COUNTER.next_serial();
                handle.set_focus(self, window.wl_surface().cloned(), serial);
                workspace.needs_redraw = true;
            }
        }

        workspace.update_layout(&self.space);
    }

    pub fn move_window(&mut self, direction: Direction) {
        debug!("{direction:?}");

        // TODO: this should be simplified !
        let new_focus = {
            let ws = self.get_current_workspace();
            let ws = ws.get();
            let (container, window) = ws.get_focus();

            match window {
                Some(window) => {
                    let target = self
                        .scan_window(direction)
                        .map(|target| target.user_data().get::<WindowState>().unwrap().id())
                        .and_then(|id| {
                            ws.root()
                                .container_having_window(id)
                                .map(|container| (id, container))
                        });

                    if let Some((target_window_id, target_container)) = target {
                        let target_container_id = target_container.get().id;
                        let current_container_id = container.get().id;

                        // Ensure we are not taking a double borrow if window moves in the same container
                        if target_container_id == current_container_id {
                            let mut container = container.get_mut();
                            container.nodes.remove(&window.id());
                            match direction {
                                Direction::Left | Direction::Up => {
                                    container.insert_window_before(target_window_id, window)
                                }
                                Direction::Right | Direction::Down => {
                                    container.insert_window_after(target_window_id, window)
                                }
                            }
                        } else {
                            let container_state = {
                                let mut target_container = target_container.get_mut();
                                let mut current = container.get_mut();
                                current.nodes.remove(&window.id());
                                match direction {
                                    Direction::Left | Direction::Up => target_container
                                        .insert_window_after(target_window_id, window),
                                    Direction::Right | Direction::Down => target_container
                                        .insert_window_before(target_window_id, window),
                                }

                                current.state()
                            };

                            if container_state == ContainerState::Empty {
                                let container = container.get();
                                if let Some(parent) = &container.parent {
                                    let mut parent = parent.get_mut();
                                    parent.nodes.remove(&container.id);
                                }
                            }
                        }

                        Some(target_container)
                    } else {
                        None
                    }
                }
                None => None,
            }
        };

        if let Some(new_focus) = new_focus {
            let ws = self.get_current_workspace();
            let mut ws = ws.get_mut();
            ws.set_container_focused(&new_focus);
        }

        let ws = self.get_current_workspace();
        let mut ws = ws.get_mut();
        ws.update_layout(&self.space);
    }
}
