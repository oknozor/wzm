use crate::shell::windows::WindowState;
use nix::libc;
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
    KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
};
use smithay::input::keyboard::FilterResult;
use smithay::input::pointer::{AxisFrame, ButtonEvent, MotionEvent};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::SERIAL_COUNTER;
use std::io;
use std::os::unix::prelude::CommandExt;
use std::process::{Command, Stdio};
use tracing::{debug, warn};
use wzm_config::action::KeyAction;
use wzm_config::keybinding::Action;
use xkbcommon::xkb::keysyms::{KEY_XF86Switch_VT_1, KEY_XF86Switch_VT_12};

use crate::state::Wzm;

impl Wzm {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => match self.keyboard_key_to_action::<I>(event) {
                KeyAction::Run(cmd, env) => spawn(cmd, env),
                KeyAction::ScaleUp => {}
                KeyAction::ScaleDown => {}
                KeyAction::RotateOutput => {}
                KeyAction::Screen(_) => {}
                KeyAction::ToggleTint => {}
                KeyAction::TogglePreview => {}
                KeyAction::ToggleFullScreenWindow => self.toggle_fullscreen_window(),
                KeyAction::ToggleFullScreenContainer => self.toggle_fullscreen_container(),
                KeyAction::MoveWindow(direction) => self.move_window(direction),
                KeyAction::MoveContainer(_) => {}
                KeyAction::MoveFocus(direction) => self.move_focus(direction),
                KeyAction::MoveToWorkspace(_) => {}
                KeyAction::LayoutVertical => self.set_layout_v(),
                KeyAction::LayoutHorizontal => self.set_layout_h(),
                KeyAction::ToggleFloating => self.toggle_floating(),
                KeyAction::VtSwitch(_) => {}
                KeyAction::CloseWindow => self.close(),
                KeyAction::Quit => {}
                KeyAction::None => {}
            },
            InputEvent::PointerMotion { .. } => {}
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.space.outputs().next().unwrap();

                let output_geo = self.space.output_geometry(output).unwrap();

                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let serial = SERIAL_COUNTER.next_serial();

                let pointer = self.seat.get_pointer().unwrap();

                let under = self.surface_under(pos);

                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event, .. } => {
                let pointer = self.seat.get_pointer().unwrap();
                let keyboard = self.seat.get_keyboard().unwrap();

                let serial = SERIAL_COUNTER.next_serial();

                let button = event.button_code();

                let button_state = event.state();

                if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
                    let maybe_under_pointer = self
                        .space
                        .element_under(pointer.current_location())
                        .map(|(w, l)| (w.clone(), l));

                    match maybe_under_pointer {
                        Some((window, _)) => {
                            let workspace_ref = self.get_current_workspace();
                            let mut ws = workspace_ref.get_mut();
                            let id = window.user_data().get::<WindowState>().unwrap().id();
                            let container = ws.root().container_having_window(id).unwrap();
                            ws.set_container_focused(&container);
                            container.get_mut().set_focus(id);

                            self.space.raise_element(&window, true);
                            keyboard.set_focus(
                                self,
                                Some(window.toplevel().unwrap().wl_surface().clone()),
                                serial,
                            );

                            self.space.elements().for_each(|window| {
                                window.toplevel().unwrap().send_pending_configure();
                            });
                        }
                        None => {
                            self.space.elements().for_each(|window| {
                                window.set_activated(false);
                                window.toplevel().unwrap().send_pending_configure();
                            });
                            keyboard.set_focus(self, Option::<WlSurface>::None, serial);
                        }
                    }
                };

                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state: button_state,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event, .. } => {
                let source = event.source();

                let horizontal_amount = event.amount(Axis::Horizontal).unwrap_or_else(|| {
                    event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.
                });
                let vertical_amount = event.amount(Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.
                });
                let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
                let vertical_amount_discrete = event.amount_v120(Axis::Vertical);

                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                if horizontal_amount != 0.0 {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);
                    if let Some(discrete) = horizontal_amount_discrete {
                        frame = frame.v120(Axis::Horizontal, discrete as i32);
                    }
                }
                if vertical_amount != 0.0 {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    if let Some(discrete) = vertical_amount_discrete {
                        frame = frame.v120(Axis::Vertical, discrete as i32);
                    }
                }

                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        frame = frame.stop(Axis::Vertical);
                    }
                }

                let pointer = self.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }

    fn keyboard_key_to_action<B: InputBackend>(&mut self, evt: B::KeyboardKeyEvent) -> KeyAction {
        let keycode = evt.key_code();
        let state = evt.state();
        let serial = SERIAL_COUNTER.next_serial();
        let time = Event::time_msec(&evt);
        let keyboard = self.seat.get_keyboard().unwrap();

        let action = keyboard
            .input(
                self,
                keycode,
                state,
                serial,
                time,
                |app_state, modifiers, key_handle| {
                    let keysym = key_handle.modified_sym();
                    if state == KeyState::Pressed {
                        let action = app_state
                            .config
                            .keybindings
                            .iter()
                            .find_map(|binding| binding.match_action(*modifiers, keysym))
                            .map(Action::into)
                            .map(FilterResult::Intercept);

                        match action {
                            None => match keysym.raw() {
                                KEY_XF86Switch_VT_1..=KEY_XF86Switch_VT_12 => {
                                    FilterResult::Intercept(KeyAction::VtSwitch(
                                        (keysym.raw() - KEY_XF86Switch_VT_1 + 1) as i32,
                                    ))
                                }
                                _ => FilterResult::Forward,
                            },
                            Some(action) => action,
                        }
                    } else {
                        FilterResult::Forward
                    }
                },
            )
            .unwrap_or(KeyAction::None);

        action
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
