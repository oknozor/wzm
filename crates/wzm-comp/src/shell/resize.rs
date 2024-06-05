use wzm_config::keybinding::{ResizeDirection, ResizeType};

use crate::shell::node::NodeId;
use crate::shell::siblings::SiblingDirection;
use crate::shell::{Orientation, Tree};

const MIN_SIZE: i32 = 100;

struct ResizeTargets {
    parent: NodeId,
    before: Option<NodeId>,
    target: NodeId,
    after: Option<NodeId>,
}

impl<T: Clone + Eq> Tree<T> {
    pub fn resize(&mut self, resize: ResizeType, direction: ResizeDirection, amount: i32) {
        let (tree_id, Some(leaf_id)) = self.focus else {
            return;
        };

        let Some(resize_targets) = self.find_resize_target(direction, &leaf_id, &tree_id) else {
            return;
        };

        self.resize_node(resize, direction, amount, resize_targets);
    }

    fn resize_node(
        &mut self,
        resize: ResizeType,
        direction: ResizeDirection,
        amount: i32,
        targets: ResizeTargets,
    ) {
        println!(
            "Resize Op {resize:?}, {direction:?}, {amount:?} for ({:?})",
            targets.target
        );

        let tree = self.get_tree(&targets.parent);
        let tree = tree.borrow();
        let child = self.nodes.get(&targets.target).unwrap();
        let (before, after) = (
            targets.before.and_then(|b| self.nodes.get(&b)),
            targets.after.and_then(|a| self.nodes.get(&a)),
        );

        let child_edge = match direction {
            ResizeDirection::Height => child.geometry().size.h,
            ResizeDirection::Width => child.geometry().size.w,
        };

        let (before_edge, after_edge) = match direction {
            ResizeDirection::Height => (
                before.map(|b| b.geometry().size.h),
                after.map(|a| a.geometry().size.h),
            ),
            ResizeDirection::Width => (
                before.map(|b| b.geometry().size.w),
                after.map(|a| a.geometry().size.w),
            ),
        };

        let total = before_edge.unwrap_or(0) + after_edge.unwrap_or(0) + child_edge;
        let edge_count = if before.is_some() && after.is_some() {
            2
        } else {
            1
        };
        let upper_limit = total - MIN_SIZE * edge_count;
        let upper_limit_edge = (total - MIN_SIZE) / edge_count;

        let child_edge = match resize {
            ResizeType::Shrink => (child_edge - amount).max(MIN_SIZE),
            ResizeType::Grow => (child_edge + amount).min(upper_limit),
        };

        let update_edge: fn(i32, i32, ResizeType, i32) -> i32 =
            |edge, step, resize, upper_limit| match resize {
                ResizeType::Shrink => (edge + step).min(upper_limit),
                ResizeType::Grow => (edge - step).max(MIN_SIZE),
            };

        let amount = amount / edge_count;

        let before_ratio = before_edge.map(|edge| {
            update_edge(edge, amount, resize, upper_limit_edge) as f32 / tree.edge() as f32
        });
        let after_ratio = after_edge.map(|edge| {
            update_edge(edge, amount, resize, upper_limit_edge) as f32 / tree.edge() as f32
        });

        if let Some((ratio, node)) = before_ratio.zip(before) {
            node.set_ratio(ratio);
        }

        if let Some((ratio, node)) = after_ratio.zip(after) {
            node.set_ratio(ratio);
        }

        let child_ratio = child_edge as f32 / tree.edge() as f32;
        child.set_ratio(child_ratio);

        drop(tree);
        self.update_geometries(&targets.parent)
    }

    fn find_resize_target(
        &mut self,
        direction: ResizeDirection,
        leaf_id: &NodeId,
        tree_id: &NodeId,
    ) -> Option<ResizeTargets> {
        let tree = self.get_tree(tree_id);
        let tree = tree.borrow();

        if tree_id == &self.root && tree.children.len() == 1 {
            return None;
        }

        let (before, after) = match direction {
            ResizeDirection::Height if tree.orientation == Orientation::Vertical => (
                self.find_sibling(leaf_id, SiblingDirection::Up),
                self.find_sibling(leaf_id, SiblingDirection::Down),
            ),
            ResizeDirection::Width if tree.orientation == Orientation::Horizontal => (
                self.find_sibling(leaf_id, SiblingDirection::Left),
                self.find_sibling(leaf_id, SiblingDirection::Right),
            ),
            _ => {
                let (leaf_id, Some(tree_id)) =
                    self.first_parent_with_orientation(leaf_id, tree.orientation.invert())
                else {
                    return None;
                };

                drop(tree);
                return self.find_resize_target(direction, &leaf_id, &tree_id);
            }
        };

        Some(ResizeTargets {
            parent: *tree_id,
            before,
            target: *leaf_id,
            after,
        })
    }
}
