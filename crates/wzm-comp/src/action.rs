use smithay::desktop::Window;
use smithay::utils::{Point, Serial, SERIAL_COUNTER};

use wzm_config::action::Direction;

use crate::shell::container::ContainerLayout;
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

        ws.update_borders();
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

        ws.update_borders();
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
            ws.update_borders();
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
}
