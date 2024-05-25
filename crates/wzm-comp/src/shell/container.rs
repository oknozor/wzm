use std::cell::{Ref, RefCell, RefMut};
use std::num::NonZeroUsize;
use std::rc::Rc;

use smithay::desktop::{Space, Window};
use smithay::utils::{Logical, Point, Rectangle, Size};

use smithay::output::Output;
use smithay::wayland::shell::xdg::ToplevelSurface;
use tracing::debug;

use crate::shell::node;
use crate::shell::node::Node;

use crate::shell::nodemap::NodeMap;
use crate::shell::windows::WindowWrap;

#[derive(Debug, Clone)]
pub struct ContainerRef {
    inner: Rc<RefCell<Container>>,
}

impl ContainerRef {
    pub fn new(container: Container) -> Self {
        ContainerRef {
            inner: Rc::new(RefCell::new(container)),
        }
    }

    pub fn get(&self) -> Ref<'_, Container> {
        self.inner.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, Container> {
        self.inner.borrow_mut()
    }

    pub fn container_having_window(&self, id: u32) -> Option<ContainerRef> {
        let this = self.get();

        if this.nodes.contains(&id) {
            Some(self.clone())
        } else {
            this.nodes
                .iter_containers()
                .find_map(|c| c.container_having_window(id))
        }
    }

    pub fn find_container_by_id(&self, id: &u32) -> Option<ContainerRef> {
        let this = self.get();
        if &this.id == id {
            Some(self.clone())
        } else {
            this.nodes
                .items
                .get(id)
                .and_then(|node| node.try_into().ok())
        }
        .or_else(|| {
            this.nodes
                .iter_containers()
                .find_map(|c| c.find_container_by_id(id))
        })
    }

    pub fn childs_containers(&self) -> Vec<ContainerRef> {
        self.get()
            .nodes
            .iter_spine()
            .filter_map(|(_, node)| match node {
                Node::Container(container) => Some(container.clone().childs_containers()),
                Node::Window(_) => None,
            })
            .flatten()
            .chain([self.clone()])
            .collect::<Vec<_>>()
    }
}

#[derive(Debug)]
pub struct Container {
    pub id: u32,
    pub location: Point<i32, Logical>,
    pub size: Size<i32, Logical>,
    pub output: Output,
    pub parent: Option<ContainerRef>,
    pub nodes: NodeMap,
    pub layout: ContainerLayout,
    pub gaps: i32,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ContainerState {
    Empty,
    HasContainersOnly,
    HasWindows,
}

#[derive(Debug, Copy, Clone)]
pub enum ContainerLayout {
    Vertical,
    Horizontal,
}

impl Container {
    pub fn close_window(&mut self) {
        let idx = self.get_focused_window().map(|window| {
            debug!("Closing window({:?})", window.id());
            window.send_close();
            window.id()
        });

        if let Some(id) = idx {
            debug!("Removing window({:?}) from the tree", id);
            let _surface = self.nodes.remove(&id);
        }
    }

    pub fn create_child(
        &mut self,
        layout: ContainerLayout,
        parent: ContainerRef,
        gaps: i32,
    ) -> ContainerRef {
        if self.nodes.spine.len() <= 1 {
            self.layout = layout;
            parent
        } else {
            let size = match self.layout {
                ContainerLayout::Vertical => (self.size.w, self.size.h / 2),
                ContainerLayout::Horizontal => (self.size.w / 2, self.size.h),
            }
            .into();

            let location = match self.layout {
                ContainerLayout::Vertical => (self.location.x, self.location.y + self.size.h),
                ContainerLayout::Horizontal => (self.location.x + self.size.w, self.location.y),
            }
            .into();

            let child = Container {
                id: node::id::next(),
                location,
                size,
                output: self.output.clone(),
                parent: Some(parent),
                nodes: NodeMap::default(),
                layout,
                gaps,
            };

            let child_ref = ContainerRef::new(child);
            if let Some(focus) = self.get_focused_window() {
                let focus_id = focus.id();
                self.nodes
                    .insert_after(focus_id, Node::Container(child_ref.clone()));
                let focus = self
                    .nodes
                    .remove(&focus_id)
                    .expect("Focused window node should exists");
                child_ref.get_mut().nodes.push(focus);
            } else {
                self.nodes.push(Node::Container(child_ref.clone()));
            }

            child_ref
        }
    }

    pub fn flatten_window(&self) -> Vec<WindowWrap> {
        let mut windows: Vec<WindowWrap> = self.nodes.iter_windows().cloned().collect();

        for child in self.nodes.iter_containers() {
            let child = child.get();
            windows.extend(child.flatten_window())
        }

        windows
    }

    fn get_child_size(&self) -> Option<Size<i32, Logical>> {
        self.nodes
            .tiled_element_len()
            .map(NonZeroUsize::get)
            .map(|len| {
                if len == 1 {
                    self.size
                } else {
                    let len = len as i32;
                    let gaps = self.gaps;
                    let total_gaps = gaps * (len - 1);
                    match self.layout {
                        ContainerLayout::Vertical => {
                            let w = self.size.w;
                            let h = (self.size.h - total_gaps) / len;
                            (w, h)
                        }
                        ContainerLayout::Horizontal => {
                            let w = (self.size.w - total_gaps) / len;
                            let h = self.size.h;
                            (w, h)
                        }
                    }
                    .into()
                }
            })
    }

    pub fn get_focus(&self) -> Option<&Node> {
        self.nodes.get_focused()
    }

    pub fn get_focused_window(&self) -> Option<WindowWrap> {
        self.nodes.get_focused().and_then(|node| match node {
            Node::Window(window) => Some(window.clone()),
            _ => None,
        })
    }

    fn get_loc_for_index(&self, idx: usize, size: Size<i32, Logical>) -> Point<i32, Logical> {
        if idx == 0 {
            self.location
        } else {
            let gaps = self.gaps;
            let pos = idx as i32;

            match self.layout {
                ContainerLayout::Vertical => {
                    let x = self.location.x;
                    let y = self.location.y + (size.h + gaps) * pos;
                    (x, y)
                }
                ContainerLayout::Horizontal => {
                    let x = self.location.x + (size.w + gaps) * pos;
                    let y = self.location.y;
                    (x, y)
                }
            }
            .into()
        }
    }

    pub fn has_container(&self) -> bool {
        self.nodes.has_container()
    }

    fn has_windows(&self) -> bool {
        self.nodes.has_window()
    }

    pub fn insert_window_after(&mut self, target_id: u32, window: WindowWrap) {
        let id = window.id();
        self.nodes.insert_after(target_id, Node::Window(window));
        self.nodes.set_focus(id);
    }

    pub fn insert_window_before(&mut self, target_id: u32, window: WindowWrap) {
        let id = window.id();
        self.nodes.insert_before(target_id, Node::Window(window));
        self.nodes.set_focus(id);
    }

    // Push a window to the tree and update the focus
    pub fn push_toplevel(&mut self, surface: ToplevelSurface) -> u32 {
        let window = Node::Window(WindowWrap::from(surface));
        match self.get_focused_window() {
            None => self.nodes.push(window),
            Some(focus) => self
                .nodes
                .insert_after(focus.id(), window)
                .expect("Should insert window"),
        }
    }

    pub fn redraw(&self, space: &mut Space<Window>) {
        let focused_window_id = self.get_focused_window().map(|window| window.id());

        for (id, node) in self.nodes.iter_spine() {
            match node {
                Node::Container(container) => {
                    let child = container.get();
                    child.redraw(space);
                }
                Node::Window(window) => {
                    let activate = Some(*id) == focused_window_id;
                    window.map(space, activate)
                }
            }
        }
    }

    pub fn reparent_orphans(&mut self) {
        let mut orphans = vec![];

        for child in self.nodes.iter_containers() {
            let mut child = child.get_mut();
            if child.nodes.iter_windows().count() == 0 {
                let children = child.nodes.drain_containers();
                orphans.extend_from_slice(children.as_slice());
            }
        }

        self.nodes.extend(orphans);
    }

    pub fn set_focus(&mut self, window_id: u32) {
        if self.nodes.get(&window_id).is_some() {
            self.nodes.set_focus(window_id)
        }
    }

    pub fn set_fullscreen_loc_and_size(&mut self, output_geometry: Rectangle<i32, Logical>) {
        let gaps = self.gaps;
        self.location = (output_geometry.loc.x + gaps, output_geometry.loc.y + gaps).into();
        self.size = (
            output_geometry.size.w - 2 * gaps,
            output_geometry.size.h - 2 * gaps,
        )
            .into();
        self.update_layout(output_geometry);
    }

    pub fn state(&self) -> ContainerState {
        if self.has_windows() {
            ContainerState::HasWindows
        } else if self.has_container() {
            ContainerState::HasContainersOnly
        } else {
            ContainerState::Empty
        }
    }

    pub fn update_layout(&mut self, output_geometry: Rectangle<i32, Logical>) -> bool {
        debug!("Update Layout for container: id={}", self.id);
        let mut redraw = self.nodes.remove_dead_windows();

        if self.nodes.spine.is_empty() {
            return false;
        }

        self.reparent_orphans();

        if let Some(size) = self.get_child_size() {
            let mut tiling_index = 0;

            for (_, node) in self.nodes.iter_spine() {
                match node {
                    Node::Container(container) => {
                        let mut child = container.get_mut();
                        child.location = self.get_loc_for_index(tiling_index, size);
                        child.size = size;
                        if child.update_layout(output_geometry) {
                            redraw = true
                        };
                        tiling_index += 1;
                    }

                    Node::Window(window) if window.is_floating() => {
                        window.update_floating(output_geometry);
                    }

                    Node::Window(window) => {
                        let loc = self.get_loc_for_index(tiling_index, size);
                        if window.update_loc_and_size(Some(size), loc) {
                            redraw = true;
                        }
                        tiling_index += 1;
                    }
                }
            }
        } else {
            // Draw floating elements only
            for (_, node) in self.nodes.iter_spine() {
                match node {
                    Node::Window(window) if window.is_floating() => {
                        if window.update_floating(output_geometry) {
                            redraw = true
                        }
                    }
                    _ => unreachable!("Container should only have floating windows"),
                }
            }
        }

        redraw
    }
}
