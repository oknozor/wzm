#![allow(irrefutable_let_patterns)]

pub use smithay::reexports::calloop::EventLoop;
use smithay::reexports::calloop::LoopSignal;
pub use smithay::reexports::wayland_server::{Display, DisplayHandle};

pub use state::Wzm;
use wzm_config::WzmConfig;

use crate::backend::Backend;
use crate::shell::workspace::WorkspaceRef;

pub mod action;
pub mod backend;
pub mod decoration;
pub mod grabs;
pub mod handlers;
pub mod input;
pub mod renderer;
pub mod shell;
pub mod state;
pub struct CalloopData {
    pub wzm: Wzm,
    pub config: WzmConfig,
    pub backend: Backend,
    pub loop_signal: LoopSignal,
}

impl CalloopData {
    pub fn start_compositor(&mut self) {
        ::std::env::set_var("WAYLAND_DISPLAY", &self.wzm.socket_name);

        if let Some(output) = self.wzm.space.outputs().next() {
            self.wzm
                .workspaces
                .insert(0, WorkspaceRef::new(output, self.config.gaps as i32));
        } else {
            panic!("Failed to create Workspace 0 on default Output");
        }

        dbg!(&self.wzm.socket_name);
    }
}

#[cfg(test)]
mod arch_test {
    use archunit_rs::rule::{ArchRuleBuilder, CheckRule};
    use archunit_rs::{ExludeModules, Structs};

    #[test]
    fn test() {
        Structs::that(ExludeModules::default())
            .have_name_matching("Container")
            .should()
            .only_have_private_fields()
            .check();
    }
}
