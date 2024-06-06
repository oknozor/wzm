use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
    KeyState, KeyboardKeyEvent, MouseButton, PointerAxisEvent, PointerButtonEvent,
};
use smithay::input::keyboard::{FilterResult, Keysym, ModifiersState};
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GrabStartData as PointerGrabStartData, MotionEvent,
};
use smithay::input::Seat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Resource;
use smithay::utils::{Serial, SERIAL_COUNTER};
use xkbcommon::xkb::keysyms::{KEY_XF86Switch_VT_1, KEY_XF86Switch_VT_12};

use wzm_config::action::KeyAction;
use wzm_config::keybinding;
use wzm_config::keybinding::Action;

use crate::action::spawn;
use crate::state::State;
use crate::Wzm;

impl Wzm {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => match self.keyboard_key_to_action::<I>(event) {
                KeyAction::Resize(kind, direction, amount) if self.state.resize_mode() => {
                    self.resize(kind, direction, amount)
                }
                KeyAction::Run(cmd, env) => spawn(cmd, env),
                KeyAction::ScaleUp => {}
                KeyAction::ScaleDown => {}
                KeyAction::RotateOutput => {}
                KeyAction::Screen(_) => {}
                KeyAction::ToggleTint => {}
                KeyAction::TogglePreview => {}
                KeyAction::ToggleFullScreenWindow => self.toggle_fullscreen_window(),
                KeyAction::ToggleFullScreenContainer => {}
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
                KeyAction::ToggleResize => self.toggle_resize(),
                KeyAction::ToggleSwitchLayout => self.toggle_layout(),
                KeyAction::Resize(..) => {
                    // Noop
                }
            },
            InputEvent::PointerMotion { .. } => {}
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.state.space.outputs().next().unwrap();
                let output_geo = self.state.space.output_geometry(output).unwrap();
                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.state.seat.get_pointer().unwrap();
                let under = self.state.surface_under(pos);

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
            InputEvent::PointerButton { event, .. } => self.handle_pointer_button::<I>(&event),
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

                let pointer = self.state.seat.get_pointer().unwrap();
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
        let keyboard = self.state.seat.get_keyboard().unwrap();
        let mode = self.state.current_mode;

        keyboard
            .input(
                self,
                keycode,
                state,
                serial,
                time,
                |app_state, modifiers, key_handle| {
                    let keysym = key_handle.modified_sym();
                    match state {
                        KeyState::Released if modifiers.alt => {
                            app_state.state.mod_pressed = false;
                            FilterResult::Forward
                        }
                        KeyState::Pressed if modifiers.alt => {
                            app_state.state.mod_pressed = true;
                            Self::key_pressed_to_action(
                                &mut app_state.state,
                                modifiers,
                                keysym,
                                mode,
                            )
                        }
                        KeyState::Pressed => Self::key_pressed_to_action(
                            &mut app_state.state,
                            modifiers,
                            keysym,
                            mode,
                        ),
                        _ => FilterResult::Forward,
                    }
                },
            )
            .unwrap_or(KeyAction::None)
    }

    fn key_pressed_to_action(
        app_state: &mut State,
        modifiers: &ModifiersState,
        keysym: Keysym,
        mode: keybinding::Mode,
    ) -> FilterResult<KeyAction> {
        let action = app_state
            .config
            .keybindings
            .iter()
            .find_map(|binding| binding.match_action(*modifiers, keysym, mode))
            .map(Action::into)
            .map(FilterResult::Intercept);

        match action {
            None => match keysym.raw() {
                KEY_XF86Switch_VT_1..=KEY_XF86Switch_VT_12 => FilterResult::Intercept(
                    KeyAction::VtSwitch((keysym.raw() - KEY_XF86Switch_VT_1 + 1) as i32),
                ),
                _ => FilterResult::Forward,
            },
            Some(action) => action,
        }
    }

    pub fn handle_pointer_button<I: InputBackend>(
        &mut self,
        event: &<I as InputBackend>::PointerButtonEvent,
    ) {
        let pointer = self.state.seat.get_pointer().unwrap();
        let keyboard = self.state.seat.get_keyboard().unwrap();
        let serial = SERIAL_COUNTER.next_serial();
        let button = event.button_code();
        let state = event.state();

        if let Some(MouseButton::Right) = event.button() {
            if ButtonState::Pressed == state && !pointer.is_grabbed() && self.state.mod_pressed {
                self.move_request_server(serial, button)
            }
        } else if ButtonState::Pressed == state && !pointer.is_grabbed() {
            let maybe_under_pointer = self
                .state
                .space
                .element_under(pointer.current_location())
                .map(|(w, l)| (w.clone(), l));

            match maybe_under_pointer {
                Some((window, _)) => {
                    let workspace = self.state.get_current_workspace();
                    let mut workspace = workspace.borrow_mut();

                    workspace.set_focus_matching(&window);

                    self.state.space.raise_element(&window, true);
                    keyboard.set_focus(
                        self,
                        Some(window.toplevel().unwrap().wl_surface().clone()),
                        serial,
                    );

                    self.state.space.elements().for_each(|window| {
                        window.toplevel().unwrap().send_pending_configure();
                    });
                }
                None => {
                    self.state.space.elements().for_each(|window| {
                        window.set_activated(false);
                        window.toplevel().unwrap().send_pending_configure();
                    });
                    keyboard.set_focus(self, Option::<WlSurface>::None, serial);
                }
            }
        }

        pointer.button(
            self,
            &ButtonEvent {
                button,
                state,
                serial,
                time: event.time_msec(),
            },
        );
        pointer.frame(self);
    }
}

pub fn check_grab(
    seat: &Seat<Wzm>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<PointerGrabStartData<Wzm>> {
    let pointer = seat.get_pointer()?;

    // Check that this surface has a click grab.
    if !pointer.has_grab(serial) {
        return None;
    }

    let start_data = pointer.grab_start_data()?;

    let (focus, _) = start_data.focus.as_ref()?;
    // If the focus was for a different surface, ignore the request.
    if !focus.id().same_client_as(&surface.id()) {
        return None;
    }

    Some(start_data)
}
