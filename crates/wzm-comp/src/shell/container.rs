use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use smithay::desktop::{Space, Window};
use smithay::output::Output;
use smithay::utils::{Logical, Point, Rectangle, Size};
use smithay::wayland::shell::xdg::ToplevelSurface;

use wzm_config::keybinding::{ResizeDirection, ResizeType};

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
    pub ratio: Option<f32>,
    pub edges: NodeEdge,
}

impl Container {
    pub fn print_recursive(&self, d: usize) {
        for _ in 0..d {
            print!(" ")
        };

        let (b, a) = {
            let e = &self.edges;
            let b = e.after.as_ref()
                .map(|e| format!("before = {}", e.id())).unwrap_or("".to_string());
            let a = e.before.as_ref().map(|e| format!("after = {}", e.id())).unwrap_or("".to_string());
            (b, a)
        };

        println!("-> Container({:?}) {}, {b} {a}", self.layout, self.id);
        for (i, n) in self.nodes.iter_spine() {
            for _ in 0..d {
                print!(" ")
            };

            match n {
                Node::Container(c) => {
                    c.get().print_recursive(d + 1);
                }
                Node::Window(w) => {
                    let (b, a) = {
                        let e = w.edges();
                        let b = e.after.as_ref()
                            .map(|e| format!("before = {}", e.id())).unwrap_or("".to_string());
                        let a = e.before.as_ref().map(|e| format!("after = {}", e.id())).unwrap_or("".to_string());
                        (b, a)
                    };
                    print!("Window({i}) {b} {a}");
                }
            }

            println!()
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ContainerState {
    Empty,
    HasContainersOnly,
    HasWindows,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

impl From<ResizeDirection> for LayoutDirection {
    fn from(value: ResizeDirection) -> Self {
        match value {
            ResizeDirection::Height => LayoutDirection::Vertical,
            ResizeDirection::Width => LayoutDirection::Horizontal,
        }
    }
}

impl Container {
    pub fn close_focused_window(&mut self) {
        if let Some(window) = self.get_focused_window() {
            window.send_close();
            let id = window.id();
            let (len, _) = self.default_child_ratio();

            let remaining_ratio = self.nodes.remove(&id)
                .and_then(|s| s.ratio())
                .map(|ratio| ratio / (len - 1) as f32);

            if let Some(remains) = remaining_ratio {
                for node in self.nodes.iter_mut() {
                    if let Some(ratio) = node.ratio() {
                        node.set_ratio(ratio + remains)
                    }
                }
            }

            self.update_layout()
        }
    }

    pub fn create_child(
        &mut self,
        direction: LayoutDirection,
        this: ContainerRef,
        gaps: i32,
    ) -> ContainerRef {
        if self.nodes.spine.len() <= 1 {
            self.layout = direction;
            this
        } else {
            let child_id = node::id::next();
            let child = Container {
                id: child_id,
                loc: (0, 0).into(),
                size: (0, 0).into(),
                output: self.output.clone(),
                parent: Some(this.clone()),
                nodes: NodeMap::default(),
                layout: direction,
                gaps,
                ratio: None,
                edges: NodeEdge::default(),
            };

            let child_ref = ContainerRef::new(child);

            // if the current node contains focus we insert the new node
            // after the focused one
            if let Some(focus) = self.get_focused_window() {
                let mut edges = focus.edges_mut();
                let (before, after) = (edges.before.clone(), edges.after.clone());
                edges.before = None;
                edges.after = None;

                let focus_id = focus.id();
                self.nodes
                    .insert_after(focus_id, Node::Container(child_ref.clone()));

                let focus = self
                    .nodes
                    .remove(&focus_id)
                    .expect("Focused window node should exists");

                let mut child_mut = child_ref.get_mut();

                child_mut.edges.before = before;
                child_mut.edges.after = after;
                child_mut.nodes.push(focus);
            } else {
                let before = self.nodes.node_before(&child_id);
                let after = self.nodes.node_after(&child_id);
                let mut child_mut = child_ref.get_mut();
                child_mut.edges.before = before;
                child_mut.edges.after = after;
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

        let id = match self.get_focused_window() {
            None => {
                self.nodes.push(node);
                window.id()
            }
            Some(focus) => self
                .nodes
                .insert_after(focus.id(), node)
                .expect("Should insert window"),
        };

        let (len, default_ratio) = self.default_child_ratio();

        let shrink_ratio = default_ratio / len as f32;

        for node in self.nodes.iter_mut() {
            if let Some(ratio) = node.ratio() {
                node.set_ratio(ratio - shrink_ratio)
            }
        }

        id
    }

    fn compute_size(&self, ratio: f32, total_items: i32) -> Size<i32, Logical> {
        let total_gaps = (total_items - 1) * self.gaps;
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

    pub fn default_child_ratio(&self) -> (i32, f32) {
        let current_len = self.nodes.tiled_element_len();
        let ratio = 1.0 / current_len as f32;
        (current_len as i32, ratio)
    }

    pub fn update_layout(&self) {
        let (total_item, default_ratio) = self.default_child_ratio();
        let mut loc = self.loc;

        for (_, node) in self.nodes.iter_spine() {
            let updated_size = match node {
                Node::Container(c) => {
                    let mut c = c.get_mut();
                    let ratio = c.ratio.unwrap_or(default_ratio);
                    let size = self.compute_size(ratio, total_item);

                    if (c.size != size) || (c.loc != loc) {
                        c.size = size;
                        c.loc = loc;
                        c.update_layout();
                    }
                    size
                }
                Node::Window(w) => {
                    let ratio = w.ratio().unwrap_or(default_ratio);
                    let size = self.compute_size(ratio, total_item);
                    w.update_loc_and_size(Some(size), loc);
                    size
                }
            };

            match self.layout {
                LayoutDirection::Vertical => loc.y += updated_size.h + self.gaps,
                LayoutDirection::Horizontal => loc.x += updated_size.w + self.gaps,
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
        let mut after: Option<Node> = None;

        let mut iter = self.nodes.iter_spine().peekable();
        while let Some((_, current)) = iter.next() {
            let before = iter.peek().map(|(_, node)| *node).cloned();

            match current {
                Node::Container(c) => {
                    let mut c = c.get_mut();
                    c.edges.before = before;
                    c.edges.after = after;
                }
                Node::Window(w) => {
                    let mut edges = w.edges_mut();
                    edges.before = before;
                    edges.after = after;
                }
            };

            after = Some(current.clone());
        }
    }

    pub fn resize(&mut self, default_ratio: f32, amount: u32, kind: ResizeType) {
        let ratio = self.ratio.unwrap_or(default_ratio);
        let step = match kind {
            ResizeType::Shrink => -(amount as f32 / 100.0),
            ResizeType::Grow => amount as f32 / 100.0,
        };

        let (before, after) = self.edges.split();
        match (before, after) {
            (Some(before), Some(after)) => {
                let before_ratio = before.ratio();
                let updated_before_ratio = before_ratio.unwrap_or(default_ratio) - step / 2.0;
                before.set_ratio(updated_before_ratio);

                let after_ratio = after.ratio();
                let updated_after_ratio = after_ratio.unwrap_or(default_ratio) - step / 2.0;
                after.set_ratio(updated_after_ratio);

                let updated_ratio = ratio + step;
                self.ratio = Some(updated_ratio);
            }
            (Some(edge), _) | (_, Some(edge)) => {
                let edge_ratio = edge.ratio();
                let updated_ratio = ratio + step;
                let updated_edge_ratio = edge_ratio.unwrap_or(default_ratio) - step;
                self.ratio = Some(updated_ratio);
                edge.set_ratio(updated_edge_ratio);
            }
            _ => {}
        }
    }
}
