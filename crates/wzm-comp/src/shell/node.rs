use std::cell::RefCell;
use std::rc::Rc;

use crate::shell::leaf::Leaf;
use smithay::utils::{Logical, Rectangle};

use crate::shell::tree::TreeNode;

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
    pub fn set_geometry(&self, geometry: Rectangle<i32, Logical>) {
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

    pub fn geometry(&self) -> Rectangle<i32, Logical> {
        match self {
            Node::Leaf(leaf) => leaf.borrow().geometry,
            Node::Tree(tree) => tree.borrow().geometry,
        }
    }

    pub fn ratio(&self) -> Option<f32> {
        match self {
            Node::Leaf(leaf) => leaf.borrow().ratio,
            Node::Tree(tree) => tree.borrow().ratio,
        }
    }

    pub fn set_ratio(&self, ratio: f32) {
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

    pub fn parent_id(&self) -> Option<NodeId> {
        match self {
            Node::Leaf(leaf) => leaf.borrow().parent,
            Node::Tree(tree) => tree.borrow().parent,
        }
    }

    pub fn set_parent_id(&self, id: &NodeId) {
        match self {
            Node::Leaf(leaf) => leaf.borrow_mut().parent = Some(*id),
            Node::Tree(tree) => tree.borrow_mut().parent = Some(*id),
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf(_))
    }
}
