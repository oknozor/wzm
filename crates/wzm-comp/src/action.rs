use crate::shell::container::ContainerLayout;
use crate::Wzm;
use tracing::debug;

impl Wzm {
    pub fn set_layout_h(&mut self) {
        debug!("set horizontal layout");
        self.next_layout = Some(ContainerLayout::Horizontal)
    }

    pub fn set_layout_v(&mut self) {
        debug!("set vertical layout");
        self.next_layout = Some(ContainerLayout::Vertical)
    }
}
