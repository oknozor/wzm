use crate::Wzm;
use smithay::desktop::Window;
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, GrabStartData as PointerGrabStartData, MotionEvent, PointerGrab,
    PointerInnerHandle, RelativeMotionEvent,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};
use tracing::debug;

pub struct MoveSurfaceGrab {
    pub start_data: PointerGrabStartData<Wzm>,
    pub window: Window,
    pub initial_window_location: Point<i32, Logical>,
}

impl PointerGrab<Wzm> for MoveSurfaceGrab {
    fn motion(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        _focus: Option<(WlSurface, Point<i32, Logical>)>,
        event: &MotionEvent,
    ) {
        // While the grab is active, no client has pointer focus
        handle.motion(data, None, event);

        let delta = event.location - self.start_data.location;
        let new_location = self.initial_window_location.to_f64() + delta;
        let ws = data.get_current_workspace();
        let ws = ws.get_mut();
        let (_, w) = ws.get_focus();
        let wrap = w.unwrap();
        wrap.update_loc(new_location.to_i32_round());
        debug!("moving window to {new_location:?}");

        wrap.map(&mut data.space, true);
    }

    fn relative_motion(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        focus: Option<(WlSurface, Point<i32, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        if !handle.current_pressed().contains(&self.start_data.button) {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        details: AxisFrame,
    ) {
        handle.axis(data, details)
    }

    fn frame(&mut self, data: &mut Wzm, handle: &mut PointerInnerHandle<'_, Wzm>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event)
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event)
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event)
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event)
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event)
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event)
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event)
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut Wzm,
        handle: &mut PointerInnerHandle<'_, Wzm>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event)
    }

    fn start_data(&self) -> &PointerGrabStartData<Wzm> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut Wzm) {}
}
