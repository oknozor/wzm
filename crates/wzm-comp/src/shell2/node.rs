use std::cell::RefCell;
use std::rc::Rc;

use smithay::utils::{Logical, Rectangle};

use crate::shell2::Orientation;

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub enum NodeId {
    Leaf(u32),
    Tree(u32),
}

pub enum Node<T> {
    Leaf(Rc<RefCell<Leaf<T>>>),
    Tree(Rc<RefCell<TreeNode>>),
}

impl<T> Node<T> {
    pub(super) fn id(&self) -> NodeId {
        match self {
            Node::Leaf(l) => l.borrow().id,
            Node::Tree(t) => t.borrow().id
        }
    }

    pub(super) fn set_geometry(&self, geometry: Rectangle<i32, Logical>) {
        match self {
            Node::Leaf(leaf) => {
                let mut leaf = leaf.borrow_mut();
                leaf.geometry = geometry;
            }
            Node::Tree(tree) => {
                let mut tree = tree.borrow_mut();
                tree.geometry = geometry;
            }
        }
    }

    pub(super) fn set_edge(&self, size: i32, orientation: Orientation) {
        match self {
            Node::Leaf(leaf) => {
                let mut leaf = leaf.borrow_mut();
                match orientation {
                    Orientation::Vertical => leaf.geometry.size.h = size,
                    Orientation::Horizontal => leaf.geometry.size.w = size,
                }
            }
            Node::Tree(tree) => {
                let mut tree = tree.borrow_mut();
                match orientation {
                    Orientation::Vertical => tree.geometry.size.h = size,
                    Orientation::Horizontal => tree.geometry.size.w = size,
                }
            }
        }
    }

    pub(super) fn edge(&self, orientation: Orientation) -> i32 {
        match orientation {
            Orientation::Vertical => self.geometry().size.h,
            Orientation::Horizontal => self.geometry().size.w
        }
    }


    pub(super) fn geometry(&self) -> Rectangle<i32, Logical> {
        match self {
            Node::Leaf(leaf) => leaf.borrow().geometry,
            Node::Tree(tree) => tree.borrow().geometry,
        }
    }

    pub(super) fn ratio(&self) -> Option<f32> {
        match self {
            Node::Leaf(leaf) => leaf.borrow().ratio,
            Node::Tree(tree) => tree.borrow().ratio,
        }
    }

    pub(super) fn set_ratio(&self, ratio: f32) {
        match self {
            Node::Leaf(leaf) => {
                let mut leaf = leaf.borrow_mut();
                leaf.ratio = Some(ratio);
            }
            Node::Tree(tree) => {
                let mut tree = tree.borrow_mut();
                tree.ratio = Some(ratio);
            }
        }
    }

    pub(super) fn parent_id(&self) -> Option<NodeId> {
        match self {
            Node::Leaf(leaf) => leaf.borrow().parent,
            Node::Tree(tree) => tree.borrow().parent,
        }
    }

    pub(super) fn set_parent_id(&self, id: &NodeId) {
        match self {
            Node::Leaf(leaf) => leaf.borrow_mut().parent = Some(*id),
            Node::Tree(tree) => tree.borrow_mut().parent = Some(*id),
        }
    }

    pub(super) fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf(_))
    }
}

pub struct Leaf<T> {
    pub(super) id: NodeId,
    pub(super) parent: Option<NodeId>,
    pub(super) geometry: Rectangle<i32, Logical>,
    pub(super) ratio: Option<f32>,
    pub(super) data: T,
}

pub struct TreeNode {
    pub(super) id: NodeId,
    pub(super) parent: Option<NodeId>,
    pub(super) children: Vec<NodeId>,
    pub(super) geometry: Rectangle<i32, Logical>,
    pub(super) ratio: Option<f32>,
    pub(super) orientation: Orientation,
}

impl TreeNode {
    pub(super) fn child_index(&self, child_id: &NodeId) -> usize {
        self.children
            .iter()
            .enumerate()
            .find(|(_, id)| *id == child_id)
            .map(|(idx, _)| idx)
            .expect("tried to get a non-existent child index")
    }

    pub(super) fn is_first_child(&self, node_id: &NodeId) -> bool {
        debug_assert!(self.children.contains(node_id));
        &self.children[0] == node_id
    }

    pub(super) fn is_last_child(&self, node_id: &NodeId) -> bool {
        debug_assert!(self.children.contains(node_id));
        &self.children[self.children.len() - 1] == node_id
    }

    pub (super) fn child_before(&self, child_id: &NodeId) -> Option<NodeId> {
        if self.is_first_child(child_id) || self.children.len() == 1 {
            None
        } else {
            let idx = self.child_index(child_id);
            Some(self.children[idx - 1])
        }
    }

    pub (super) fn child_after(&self, child_id: &NodeId) -> Option<NodeId> {
        if self.is_last_child(child_id) || self.children.len() == 1 {
            None
        } else {
            let idx = self.child_index(child_id);
            Some(self.children[idx + 1])
        }
    }

    pub(super) fn has_leaf(&self) -> bool {
        self.children.iter().any(|id| matches!(id, NodeId::Leaf(_)))
    }

    pub(super) fn edge(&self) -> i32 {
        match self.orientation {
            Orientation::Vertical => self.geometry.size.h,
            Orientation::Horizontal => self.geometry.size.w,
        }
    }
}
