#![allow(irrefutable_let_patterns)]

pub mod action;
pub mod grabs;
pub mod handlers;
pub mod input;
pub mod shell;
pub mod state;
pub mod winit;
pub use smithay::reexports::calloop::EventLoop;
pub use smithay::reexports::wayland_server::{Display, DisplayHandle};
pub use state::Wzm;

pub struct CalloopData {
    pub state: Wzm,
    pub display_handle: DisplayHandle,
}
