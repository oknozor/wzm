use smithay::utils::IsAlive;
use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::num::NonZeroUsize;

use crate::shell::container::ContainerRef;
use crate::shell::node::Node;
use crate::shell::windows::WindowWrap;

#[derive(Debug, Default)]
pub struct NodeMap {
    // The node map
    pub items: HashMap<u32, Node>,
    // Node ids by their drawing order
    // TODO: consider introducing a NodeIdType here
    pub spine: Vec<u32>,
    // Store the id of the focused window
    focus_idx: Option<usize>,
}

impl NodeMap {
    pub fn iter_spine(&self) -> impl Iterator<Item = (&u32, &Node)> {
        self.spine.iter().map(|id| {
            let node = self.items.get(id).unwrap();
            (id, node)
        })
    }

    pub fn iter_windows(&self) -> impl Iterator<Item = &WindowWrap> {
        self.items.values().filter_map(|node| match node {
            Node::Window(w) => Some(w),
            _ => None,
        })
    }

    pub fn iter_containers(&self) -> impl Iterator<Item = &ContainerRef> {
        self.items.values().filter_map(|node| match node {
            Node::Container(c) => Some(c),
            _ => None,
        })
    }

    pub fn window_count(&self) -> i32 {
        self.iter_windows().count() as i32
    }

    pub fn container_count(&self) -> i32 {
        self.iter_containers().count() as i32
    }

    pub fn drain_containers(&mut self) -> Vec<(u32, Node)> {
        let ids: Vec<u32> = self
            .items
            .iter()
            .filter(|(_k, v)| v.is_container())
            .map(|(id, _n)| id)
            .cloned()
            .collect();

        let mut drained = vec![];

        for id in ids {
            // WARN: Might be kroken
            self.spine.retain(|id_| *id_ != id);
            let node = self.items.remove(&id).unwrap();
            drained.push((id, node))
        }

        drained
    }

    pub fn remove_dead_windows(&mut self) -> bool {
        let ids: Vec<u32> = self
            .items
            .iter()
            .filter_map(|(_k, v)| v.try_into().ok())
            .filter(|window: &WindowWrap| !window.inner().alive())
            .map(|window| window.id())
            .collect();

        let redraw = !ids.is_empty();

        for id in ids {
            self.spine.retain(|id_| *id_ != id);
            let _node = self.items.remove(&id).unwrap();
        }

        redraw
    }

    pub fn drain_all(&mut self) -> Vec<(u32, Node)> {
        let mut drained = vec![];
        for id in &self.spine {
            let node = self.items.remove(id).unwrap();
            drained.push((*id, node))
        }

        for node in &mut self.items.values() {
            if let Node::Container(c) = node {
                let mut ref_mut = c.get_mut();
                drained.extend(ref_mut.nodes.drain_all());
            }
        }

        drained
    }

    pub fn extend(&mut self, other: Vec<(u32, Node)>) {
        let ids: Vec<u32> = other.iter().map(|(id, _)| *id).collect();
        self.spine.extend_from_slice(ids.as_slice());
        self.items.extend(other)
    }

    pub fn contains(&self, id: &u32) -> bool {
        self.spine.contains(id)
    }

    pub fn has_container(&self) -> bool {
        self.items.iter().any(|(_i, c)| c.is_container())
    }

    pub fn has_window(&self) -> bool {
        self.items.iter().any(|(_i, c)| !c.is_container())
    }

    pub fn get(&self, id: &u32) -> Option<&Node> {
        self.items.get(id)
    }

    pub fn get_mut(&mut self, id: &u32) -> Option<&mut Node> {
        self.items.get_mut(id)
    }

    /// Insert a container or a window in the tree and return its id
    pub fn push(&mut self, node: Node) -> u32 {
        let id = node.id();
        self.spine.push(id);

        if !node.is_container() {
            self.set_focus_index(self.spine.len() - 1);
        }

        self.items.insert(id, node);
        id
    }

    /// Insert a container or a window after the given node id in the spine
    pub fn insert_after(&mut self, id: u32, node: Node) -> Option<u32> {
        let focus_index = self.spine_index(id);

        if let Some(index) = focus_index {
            let index = index + 1;
            self.spine.insert(index, node.id());

            if !node.is_container() {
                self.set_focus_index(index);
            }

            self.items.insert(node.id(), node);
            Some(id)
        } else {
            None
        }
    }

    /// Insert a container or a window after the given node id in the spine
    pub fn insert_before(&mut self, id: u32, node: Node) -> Option<u32> {
        let focus_index = self.spine_index(id);

        if let Some(index) = focus_index {
            self.spine.insert(index, node.id());

            if !node.is_container() {
                self.set_focus_index(index);
            }

            self.items.insert(node.id(), node);
            Some(id)
        } else {
            None
        }
    }

    pub fn remove(&mut self, id: &u32) -> Option<Node> {
        self.remove_from_spine(id)
            .and_then(|id| self.items.remove(&id))
    }

    pub fn tiled_element_len(&self) -> Option<NonZeroUsize> {
        let len = self
            .items
            .values()
            .filter(|node| match node {
                Node::Container(_) => true,
                Node::Window(w) if !w.is_floating() => true,
                _ => false,
            })
            .count();

        NonZeroUsize::new(len)
    }

    pub fn iter(&self) -> Iter<'_, u32, Node> {
        self.items.iter()
    }

    fn remove_from_spine(&mut self, id: &u32) -> Option<u32> {
        // Find the matching id in spine
        let spine_part = {
            let parts = self.spine.iter().enumerate().find(|(_idx, id_)| *id_ == id);

            parts.map(|(idx, id)| (idx, *id))
        };

        if let Some((idx, id)) = spine_part {
            self.spine.remove(idx);

            if self.spine.is_empty() {
                self.focus_idx = None
            } else {
                let new_focus = self.spine[..idx]
                    .iter()
                    .enumerate()
                    .rfind(|(_idx, id)| matches!(self.items.get(id), Some(Node::Window(_))))
                    .map(|(idx, _)| idx);

                if let Some(new_focus) = new_focus {
                    self.set_focus_index(new_focus)
                }
            }
            Some(id)
        } else {
            None
        }
    }

    pub fn set_focus(&mut self, id: u32) {
        let new_focus = self
            .spine
            .iter()
            .enumerate()
            .find(|(_, id_)| **id_ == id)
            .map(|(idx, _)| idx);

        if let Some(new_focus) = new_focus {
            self.set_focus_index(new_focus);
        }
    }

    pub fn get_focused(&self) -> Option<&Node> {
        self.focus_idx
            .and_then(|idx| self.spine.get(idx))
            .and_then(|id| self.items.get(id))
            .or_else(|| {
                self.iter_windows()
                    .last()
                    .and_then(|window| self.items.get(&window.id()))
            })
    }

    fn set_focus_index(&mut self, idx: usize) {
        debug_assert!(self.spine.get(idx).is_some());
        self.focus_idx = Some(idx)
    }

    fn spine_index(&mut self, id: u32) -> Option<usize> {
        self.spine
            .iter()
            .enumerate()
            .find(|(_, node_id)| **node_id == id)
            .map(|(idx, _)| idx)
    }
}
