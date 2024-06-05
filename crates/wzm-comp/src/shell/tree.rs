use smithay::utils::{Logical, Rectangle};

use crate::shell::node::NodeId;
use crate::shell::Orientation;

pub struct TreeNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub geometry: Rectangle<i32, Logical>,
    pub ratio: Option<f32>,
    pub orientation: Orientation,
}

impl TreeNode {
    pub fn child_index(&self, child_id: &NodeId) -> usize {
        self.children
            .iter()
            .enumerate()
            .find(|(_, id)| *id == child_id)
            .map(|(idx, _)| idx)
            .expect("tried to get a non-existent child index")
    }

    pub fn is_first_child(&self, node_id: &NodeId) -> bool {
        debug_assert!(self.children.contains(node_id));
        &self.children[0] == node_id
    }

    pub fn is_last_child(&self, node_id: &NodeId) -> bool {
        debug_assert!(self.children.contains(node_id));
        &self.children[self.children.len() - 1] == node_id
    }

    pub fn child_before(&self, child_id: &NodeId) -> Option<NodeId> {
        if self.is_first_child(child_id) || self.children.len() == 1 {
            None
        } else {
            let idx = self.child_index(child_id);
            Some(self.children[idx - 1])
        }
    }

    pub fn child_after(&self, child_id: &NodeId) -> Option<NodeId> {
        if self.is_last_child(child_id) || self.children.len() == 1 {
            None
        } else {
            let idx = self.child_index(child_id);
            Some(self.children[idx + 1])
        }
    }

    pub fn has_leaf(&self) -> bool {
        self.children.iter().any(|id| matches!(id, NodeId::Leaf(_)))
    }

    pub fn edge(&self) -> i32 {
        match self.orientation {
            Orientation::Vertical => self.geometry.size.h,
            Orientation::Horizontal => self.geometry.size.w,
        }
    }
}
