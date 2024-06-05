use crate::shell::node::NodeId;
use crate::shell::{Orientation, Tree};

pub(super) enum SiblingDirection {
    Left,
    Right,
    Up,
    Down,
}

impl<T: Clone + Eq> Tree<T> {
    pub(super) fn find_sibling(
        &self,
        node_id: &NodeId,
        direction: SiblingDirection,
    ) -> Option<NodeId> {
        let node = self.nodes.get(node_id).expect("existing node");
        let parent_id = node.parent_id()?;
        let parent = self.get_tree(&parent_id);
        let parent = parent.borrow();

        match parent.orientation {
            Orientation::Vertical => match direction {
                SiblingDirection::Up if !parent.is_first_child(node_id) => {
                    parent.child_before(node_id)
                }
                SiblingDirection::Down if !parent.is_last_child(node_id) => {
                    parent.child_after(node_id)
                }
                _ => self.find_sibling(&parent_id, direction),
            },
            Orientation::Horizontal => match direction {
                SiblingDirection::Left if !parent.is_first_child(node_id) => {
                    parent.child_before(node_id)
                }
                SiblingDirection::Right if !parent.is_last_child(node_id) => {
                    parent.child_after(node_id)
                }
                _ => self.find_sibling(&parent_id, direction),
            },
        }
    }

    pub(super) fn first_parent_with_orientation(
        &self,
        node_id: &NodeId,
        orientation: Orientation,
    ) -> (NodeId, Option<NodeId>) {
        let node = self.nodes.get(node_id).expect("existing node");
        let Some(parent_id) = node.parent_id() else {
            return (*node_id, None);
        };

        let parent = self.get_tree(&parent_id);
        let parent = parent.borrow();

        if parent.orientation == orientation {
            (*node_id, Some(parent_id))
        } else {
            self.first_parent_with_orientation(&parent_id, orientation)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::shell::node::NodeId;
    use crate::shell::siblings::SiblingDirection;
    use crate::shell::{Orientation, Tree};
    use sealed_test::prelude::*;

    #[sealed_test]
    fn get_siblings() {
        let mut tree = Tree::new(Default::default(), Orientation::Horizontal);

        tree.insert(());
        tree.insert(());
        tree.split_insert((), Orientation::Vertical);
        tree.set_focus((NodeId::Tree(1), NodeId::Leaf(2)));
        tree.split_insert((), Orientation::Vertical);
        tree.set_focus((NodeId::Tree(6), NodeId::Leaf(7)));
        tree.split_insert((), Orientation::Horizontal);

        //      1
        //     / \
        //    /   \
        //   6     4
        //  / \   / \
        // 2   8 3   5
        //    / \
        //   7   9
        let tree1 = NodeId::Tree(1);
        let leaf2 = NodeId::Leaf(2);
        let leaf3 = NodeId::Leaf(3);
        let tree4 = NodeId::Tree(4);
        let leaf5 = NodeId::Leaf(5);
        let tree6 = NodeId::Tree(6);
        let leaf7 = NodeId::Leaf(7);
        let tree8 = NodeId::Tree(8);
        let leaf9 = NodeId::Leaf(9);

        let find_siblings = |id| {
            (
                tree.find_sibling(id, SiblingDirection::Left),
                tree.find_sibling(id, SiblingDirection::Up),
                tree.find_sibling(id, SiblingDirection::Right),
                tree.find_sibling(id, SiblingDirection::Down),
            )
        };

        assert_eq!(find_siblings(&tree1), (None, None, None, None));
        assert_eq!(
            find_siblings(&leaf2),
            (None, None, Some(tree4), Some(tree8))
        );
        assert_eq!(
            find_siblings(&leaf3),
            (Some(tree6), None, None, Some(leaf5))
        );
        assert_eq!(find_siblings(&tree4), (Some(tree6), None, None, None));
        assert_eq!(
            find_siblings(&leaf5),
            (Some(tree6), Some(leaf3), None, None)
        );
        assert_eq!(find_siblings(&tree6), (None, None, Some(tree4), None));
        assert_eq!(
            find_siblings(&leaf7),
            (None, Some(leaf2), Some(leaf9), None)
        );
        assert_eq!(
            find_siblings(&tree8),
            (None, Some(leaf2), Some(tree4), None)
        );
        assert_eq!(
            find_siblings(&leaf9),
            (Some(leaf7), Some(leaf2), Some(tree4), None)
        );
    }

    #[sealed_test]
    fn get_first_parent_with_inverted_orientation() {
        let mut tree = Tree::new(Default::default(), Orientation::Horizontal);

        tree.insert(());
        tree.insert(());
        tree.split_insert((), Orientation::Vertical);
        tree.split_insert((), Orientation::Vertical);

        //      1 H
        //     / \
        //    /   \
        //   2     4 V
        //        / \
        //       3   6 V
        //          / \
        //         5   7
        let tree1 = NodeId::Tree(1);
        let tree4 = NodeId::Tree(4);
        let leaf7 = NodeId::Leaf(7);

        let (ancestor, horizontal_parent) =
            tree.first_parent_with_orientation(&leaf7, Orientation::Horizontal);
        assert_eq!(ancestor, tree4);
        assert_eq!(horizontal_parent, Some(tree1));
    }
}
