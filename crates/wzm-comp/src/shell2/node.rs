use std::cell::RefCell;
use std::rc::Rc;

use smithay::utils::{Logical, Rectangle};
use crate::shell2::leaf::Leaf;

use crate::shell2::Orientation;
use crate::shell2::tree::TreeNode;

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug)]
pub enum NodeId {
    Leaf(u32),
    Tree(u32),
}

impl NodeId {
    pub fn value(&self) -> u32 {
       match self {
           NodeId::Leaf(id) => *id,
           NodeId::Tree(id) => *id
       }
    }
}

pub enum Node<T> {
    Leaf(Rc<RefCell<Leaf<T>>>),
    Tree(Rc<RefCell<TreeNode>>),
}

impl<T> Node<T> {
    pub fn id(&self) -> NodeId {
        match self {
            Node::Leaf(l) => l.borrow().id,
            Node::Tree(t) => t.borrow().id
        }
    }

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

    pub fn set_edge(&self, size: i32, orientation: Orientation) {
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

    pub fn edge(&self, orientation: Orientation) -> i32 {
        match orientation {
            Orientation::Vertical => self.geometry().size.h,
            Orientation::Horizontal => self.geometry().size.w
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
