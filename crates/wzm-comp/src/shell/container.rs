use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use smithay::desktop::{Space, Window};
use smithay::utils::{Logical, Point, Rectangle, Size};

use smithay::output::Output;
use smithay::wayland::shell::xdg::ToplevelSurface;
use tracing::debug;

use crate::shell::node;
use crate::shell::node::{Node, NodeEdge};

use crate::shell::nodemap::NodeMap;
use crate::shell::windows::WzmWindow;

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
    pub loc: Point<i32, Logical>,
    pub size: Size<i32, Logical>,
    pub output: Output,
    pub parent: Option<ContainerRef>,
    pub nodes: NodeMap,
    pub layout: LayoutDirection,
    pub gaps: i32,
    pub edges: NodeEdge,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ContainerState {
    Empty,
    HasContainersOnly,
    HasWindows,
}

#[derive(Debug, Copy, Clone)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

impl Container {
    pub fn close_focused_window(&mut self) {
        if let Some(window) = self.get_focused_window() {
            window.send_close();
            let id = window.id();
            debug!("Removing window({:?}) from the tree", id);
            let _surface = self.nodes.remove(&id);
            self.update_layout()
        }
    }

    pub fn create_child(
        &mut self,
        direction: LayoutDirection,
        parent: ContainerRef,
        gaps: i32,
    ) -> ContainerRef {
        if self.nodes.spine.len() <= 1 {
            self.layout = direction;
            parent
        } else {
            let child_id = node::id::next();

            let child = Container {
                id: child_id,
                loc: (0, 0).into(),
                size: (0, 0).into(),
                output: self.output.clone(),
                parent: Some(parent.clone()),
                nodes: NodeMap::default(),
                layout: direction,
                gaps,
                edges: NodeEdge::default(),
            };

            let child_ref = ContainerRef::new(child);

            // if the current node contains focus we insert the new node
            // after the focused one
            if let Some(focus) = self.get_focused_window() {
                let focus_id = focus.id();
                self.nodes
                    .insert_after(focus_id, Node::Container(child_ref.clone()));

                let focus = self
                    .nodes
                    .remove(&focus_id)
                    .expect("Focused window node should exists");

                let before = self.nodes.node_before(&child_id);
                let mut child_mut = child_ref.get_mut();

                match self.layout {
                    LayoutDirection::Vertical => {
                        child_mut.edges.up = Some(focus.clone());
                        child_mut.edges.down = before;
                        child_mut.edges.left.clone_from(&self.edges.left);
                        child_mut.edges.right.clone_from(&self.edges.right);
                    }

                    LayoutDirection::Horizontal => {
                        child_mut.edges.left = Some(focus.clone());
                        child_mut.edges.right = before;
                        child_mut.edges.up.clone_from(&self.edges.up);
                        child_mut.edges.down.clone_from(&self.edges.down);
                    }
                }

                child_mut.nodes.push(focus);
            } else {
                // otherwise append the new node
                self.nodes.push(Node::Container(child_ref.clone()));
            }

            child_ref
        }
    }

    pub fn flatten_window(&self) -> Vec<WzmWindow> {
        let mut windows: Vec<WzmWindow> = self.nodes.iter_windows().cloned().collect();

        for child in self.nodes.iter_containers() {
            let child = child.get();
            windows.extend(child.flatten_window())
        }

        windows
    }

    pub fn get_focus(&self) -> Option<&Node> {
        self.nodes.get_focused()
    }

    pub fn get_focused_window(&self) -> Option<WzmWindow> {
        self.nodes.get_focused().and_then(|node| match node {
            Node::Window(window) => Some(window.clone()),
            _ => None,
        })
    }

    pub fn has_container(&self) -> bool {
        self.nodes.has_container()
    }

    fn has_windows(&self) -> bool {
        self.nodes.has_window()
    }

    pub fn insert_window_after(&mut self, target_id: u32, window: WzmWindow) {
        let id = window.id();
        self.nodes.insert_after(target_id, Node::Window(window));
        self.nodes.set_focus(id);
    }

    pub fn insert_window_before(&mut self, target_id: u32, window: WzmWindow) {
        let id = window.id();
        self.nodes.insert_before(target_id, Node::Window(window));
        self.nodes.set_focus(id);
    }

    // Push a window to the tree and update the focus
    pub fn push_toplevel(&mut self, surface: ToplevelSurface) -> u32 {
        let window = WzmWindow::from(surface);
        let node = Node::Window(window.clone());

        match self.get_focused_window() {
            None => {
                self.nodes.push(node);
                window.id()
            }
            Some(focus) => self
                .nodes
                .insert_after(focus.id(), node)
                .expect("Should insert window"),
        }
    }

    fn get_base_size(&self) -> Size<i32, Logical> {
        let current_len = self.nodes.tiled_element_len();
        let ratio = 1.0 / current_len as f32;
        let total_gaps = (current_len - 1) as i32 * self.gaps;
        let base_size: Size<i32, Logical> = match self.layout {
            LayoutDirection::Vertical => {
                let w = self.size.w;
                let h = ((self.size.h - total_gaps) as f32 * ratio) as i32;
                (w, h).into()
            }
            LayoutDirection::Horizontal => {
                let w = ((self.size.w - total_gaps) as f32 * ratio) as i32;
                let h = self.size.h;
                (w, h).into()
            }
        };
        base_size
    }

    pub fn update_layout(&self) {
        let base_size = self.get_base_size();
        let mut loc = self.loc;

        for (_, node) in self.nodes.iter_spine() {
            match node {
                Node::Container(c) => {
                    let mut c = c.get_mut();
                    if (c.size != base_size) || (c.loc != loc) {
                        c.size = base_size;
                        c.loc = loc;
                        c.update_layout();
                    }
                }
                Node::Window(w) => {
                    w.update_loc_and_size(Some(base_size), loc);
                }
            }

            match self.layout {
                LayoutDirection::Vertical => loc.y += base_size.h + self.gaps,
                LayoutDirection::Horizontal => loc.x += base_size.w + self.gaps,
            }
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

    pub fn set_fullscreen_loc_and_size(&mut self, zone: Rectangle<i32, Logical>) {
        let gaps = self.gaps;
        self.loc = (zone.loc.x + gaps, zone.loc.y + gaps).into();
        self.size = (zone.size.w - 2 * gaps, zone.size.h - 2 * gaps).into();
        // self.update_layout(zone);
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

    pub fn update_inner_edges(&mut self) {
        let mut before: Option<Node> = None;

        let mut iter = self.nodes.iter_spine().peekable();
        while let Some((_, current)) = iter.next() {
            let after = iter.peek().map(|(_, node)| *node).cloned();

            match self.layout {
                LayoutDirection::Vertical => match current {
                    Node::Container(c) => {
                        let mut c = c.get_mut();
                        c.edges.right.clone_from(&self.edges.right);
                        c.edges.left.clone_from(&self.edges.right);
                        c.edges.up = before;
                        c.edges.down = after;
                    }
                    Node::Window(w) => {
                        let mut edges = w.edges_mut();
                        edges.right.clone_from(&self.edges.right);
                        edges.left.clone_from(&self.edges.right);
                        edges.up = before;
                        edges.down = after;
                    }
                },
                LayoutDirection::Horizontal => match current {
                    Node::Container(c) => {
                        let mut c = c.get_mut();
                        c.edges.left = before;
                        c.edges.right = after;
                        c.edges.up.clone_from(&self.edges.up);
                        c.edges.down.clone_from(&self.edges.down);
                    }
                    Node::Window(w) => {
                        let mut edges = w.edges_mut();
                        edges.left = before;
                        edges.right = after;
                        edges.up.clone_from(&self.edges.up);
                        edges.down.clone_from(&self.edges.down);
                    }
                },
            }

            before = Some(current.clone());
        }
    }
}
