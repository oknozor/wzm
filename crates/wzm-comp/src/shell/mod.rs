use crate::shell::workspace::WorkspaceRef;
use crate::Wzm;

pub mod container;
pub mod node;
pub mod nodemap;
pub mod windows;
pub mod workspace;
pub const BLUE: (f32, f32, f32) = (26.0 / 255.0, 95.0 / 255.0, 205.0 / 255.0);
pub const RED: (f32, f32, f32) = (1.0, 95.0 / 255.0, 205.0 / 255.0);

pub const TILING_Z_INDEX: u8 = 100;
pub const BORDER_Z_INDEX: u8 = 102;
pub const POP_UP_Z_INDEX: u8 = 103;
pub const FLOATING_Z_INDEX: u8 = 104;
pub const CURSOR_Z_INDEX: u8 = 255;

impl Wzm {
    pub fn get_current_workspace(&self) -> WorkspaceRef {
        let current = &self.current_workspace;
        self.workspaces
            .get(current)
            .expect("Current workspace should exist")
            .clone()
    }

    pub fn move_to_workspace(&mut self, num: u8) {
        // Target workspace is already focused
        if self.current_workspace == num {
            return;
        }

        let current_workspace = self.get_current_workspace();
        let mut current_workspace = current_workspace.get_mut();
        current_workspace.unmap_all(&mut self.space);
        self.current_workspace = num;

        match self.workspaces.get(&num) {
            None => {
                let output = self.space.outputs().next().unwrap();
                let workspace =
                    WorkspaceRef::new(output.clone(), &self.space, self.config.gaps as i32);
                self.workspaces.insert(num, workspace);
            }
            Some(workspace) => {
                let mut workspace = workspace.get_mut();
                workspace.update_layout(&self.space);
                workspace.needs_redraw = true;
            }
        };
    }
}
