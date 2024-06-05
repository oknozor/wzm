use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use smithay::utils::{Logical, Rectangle};

use leaf::Leaf;
use tree::TreeNode;

use crate::shell::node::{Node, NodeId};

mod leaf;
mod node;
mod tree;

mod resize;
mod siblings;

pub struct Tree<T> {
    nodes: BTreeMap<NodeId, Node<T>>,
    root: NodeId,
    focus: (NodeId, Option<NodeId>),
    pending_update: Vec<NodeId>,
}

pub enum Direction {
    Before,
    After,
}

pub enum Resize {
    Grow,
    Shrink,
}

impl Direction {
    pub(super) fn invert(&self) -> Direction {
        match self {
            Direction::Before => Direction::After,
            Direction::After => Direction::Before,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl Orientation {
    fn invert(&self) -> Self {
        match self {
            Orientation::Vertical => Orientation::Horizontal,
            Orientation::Horizontal => Orientation::Vertical,
        }
    }
}

pub mod id {
    use std::sync::{Arc, Mutex};

    use once_cell::sync::Lazy;

    static NODE_ID_COUNTER: Lazy<Arc<Mutex<u32>>> = Lazy::new(|| Arc::new(Mutex::new(0)));

    pub fn next() -> u32 {
        let mut id = NODE_ID_COUNTER.lock().unwrap();
        *id += 1;
        *id
    }
}

impl<T: Clone + Eq> Tree<T> {
    pub(crate) fn new(geometry: Rectangle<i32, Logical>, orientation: Orientation) -> Self {
        let mut nodes = BTreeMap::new();
        let root_id = NodeId::Tree(id::next());

        let root = Node::Tree(Rc::new(RefCell::new(TreeNode {
            id: root_id,
            parent: None,
            children: vec![],
            geometry,
            ratio: None,
            orientation,
        })));

        nodes.insert(root_id, root);

        Tree {
            nodes,
            root: root_id,
            focus: (root_id, None),
            pending_update: vec![],
        }
    }

    pub(crate) fn toggle_layout(&mut self) {
        let (focused_node, _) = self.focus;
        let node = self.get_tree(&focused_node);
        let mut node = node.borrow_mut();
        node.orientation = node.orientation.invert();
        drop(node);
        self.update_geometries(&focused_node);
    }

    pub(crate) fn move_node(&mut self, target_node_id: NodeId, target_leaf_id: NodeId) {
        let (focused_node, Some(leaf_id)) = self.focus else {
            return;
        };

        let tree = self.get_tree(&focused_node);
        let mut tree = tree.borrow_mut();
        let focus_idx = tree.child_index(&leaf_id);

        if focused_node == target_node_id {
            let target_idx = tree.child_index(&target_leaf_id);
            tree.children.swap(target_idx, focus_idx);
            drop(tree);
            self.update_geometries(&focused_node);
        } else {
            let target_node = self.get_tree(&target_node_id);
            let mut target_node = target_node.borrow_mut();
            let target_idx = target_node.child_index(&target_leaf_id);
            target_node.children[target_idx] = leaf_id;
            tree.children[focus_idx] = target_leaf_id;
            self.get_leaf(&leaf_id).borrow_mut().parent = Some(target_node_id);
            self.get_leaf(&target_leaf_id).borrow_mut().parent = Some(focused_node);
            drop(tree);
            drop(target_node);
            self.focus = (target_node_id, Some(leaf_id));
            self.update_geometries(&target_node_id);
            self.update_geometries(&focused_node);
        }
    }

    pub fn get_pending_updates(&mut self) -> Vec<(T, Rectangle<i32, Logical>, bool)> {
        let ids: Vec<_> = self.pending_update.drain(..).collect();
        let focus = self.focus.1;

        ids.iter()
            .filter_map(|id| self.nodes.get(id))
            .filter(|n| n.is_leaf())
            .filter_map(|n| match n {
                Node::Leaf(l) => {
                    let leaf = l.borrow();
                    let activate = focus.map(|id| leaf.id == id).unwrap_or_default();
                    let data = leaf.data.clone();
                    let geometry = leaf.geometry;

                    Some((data, geometry, activate))
                }
                Node::Tree(_) => None,
            })
            .collect()
    }

    pub(crate) fn set_focus_matching(&mut self, data: &T) {
        let location = self
            .get_node_for_data(data)
            .map(|(tree, leaf)| (tree, Some(leaf)));

        if let Some((parent, id)) = location {
            self.focus = (parent, id)
        }
    }

    pub(crate) fn set_focus(&mut self, (tree, leaf): (NodeId, NodeId)) {
        if let Some(leaf) = self.focus.1 {
            self.pending_update.push(leaf);
        }

        self.focus = (tree, Some(leaf));
        self.pending_update.push(leaf);
    }

    pub(crate) fn get_node_for_data(&mut self, data: &T) -> Option<(NodeId, NodeId)> {
        self.nodes
            .values()
            .filter_map(|node| match node {
                Node::Leaf(leaf) => Some(leaf.borrow()),
                Node::Tree(_) => None,
            })
            .find(|leaf| &leaf.data == data)
            .map(|leaf| (leaf.parent.expect("leaf parent"), leaf.id))
    }

    pub(crate) fn get_focus(&self) -> Option<T> {
        let (_, leaf_id) = self.focus;
        let leaf = self.get_leaf(&leaf_id?);
        Some(leaf.borrow().data.clone())
    }

    /// Insert a new leaf on the tree, after the focused node.
    /// If the focused leaf is not found, append to the tree.
    pub(crate) fn insert(&mut self, data: T) {
        #[cfg(not(test))]
        debug_assert!(self.pending_update.is_empty());

        let (tree_id, leaf_id) = self.focus;
        let new_leaf_id = NodeId::Leaf(id::next());

        let tree = self.get_tree(&tree_id).clone();
        let mut tree = tree.borrow_mut();

        let node = Leaf {
            id: new_leaf_id,
            parent: Some(tree_id),
            geometry: Default::default(),
            ratio: None,
            data,
        };

        self.nodes
            .insert(new_leaf_id, Node::Leaf(Rc::new(RefCell::new(node))));

        match leaf_id {
            None => tree.children.push(new_leaf_id),
            Some(leaf_id) => {
                let leaf_idx = tree.child_index(&leaf_id);
                if leaf_idx + 1 > tree.children.len() - 1 {
                    tree.children.push(new_leaf_id)
                } else {
                    tree.children.insert(leaf_idx + 1, new_leaf_id);
                }
            }
        };

        drop(tree);
        self.focus.1 = Some(new_leaf_id);
        let focus = self.focus.0;
        self.update_geometries(&focus);
        debug_assert!(self.focus.1.is_some())
    }

    /// Create a subtree with the given orientation, reposition the
    /// focused leaf in the new tree and append the new leaf
    pub(crate) fn split_insert(&mut self, data: T, orientation: Orientation) {
        #[cfg(not(test))]
        debug_assert!(self.pending_update.is_empty());
        let (tree_id, leaf_id) = self.focus;

        // Empty root, we just need to change the orientation of root
        // and perform an insertion
        let Some(leaf_id) = leaf_id else {
            let root = self.get_root().clone();
            let mut root = root.borrow_mut();
            root.orientation = orientation;
            drop(root);
            let root = self.root;
            self.update_geometries(&root);
            self.insert(data);
            return;
        };

        let new_node_id = NodeId::Tree(id::next());
        let new_leaf_id = NodeId::Leaf(id::next());

        let new_node = TreeNode {
            id: new_node_id,
            parent: Some(tree_id),
            children: vec![leaf_id, new_leaf_id],
            geometry: Default::default(),
            ratio: None,
            orientation,
        };

        let new_leaf = Leaf {
            id: new_leaf_id,
            parent: Some(new_node_id),
            geometry: Default::default(),
            ratio: None,
            data,
        };

        let leaf = self.get_leaf(&leaf_id);
        let mut leaf = leaf.borrow_mut();
        leaf.parent = Some(new_node_id);
        leaf.ratio = None;
        drop(leaf);

        let tree = self.get_tree(&tree_id).clone();
        let mut tree = tree.borrow_mut();
        let leaf_idx = tree.child_index(&leaf_id);
        if leaf_idx + 1 > tree.children.len() - 1 {
            tree.children.push(new_node_id)
        } else {
            tree.children.insert(leaf_idx + 1, new_node_id);
        };

        tree.children.remove(leaf_idx);

        self.nodes
            .insert(new_node_id, Node::Tree(Rc::new(RefCell::new(new_node))));
        self.nodes
            .insert(new_leaf_id, Node::Leaf(Rc::new(RefCell::new(new_leaf))));

        self.focus = (new_node_id, Some(new_leaf_id));
        drop(tree);
        self.update_geometries(&tree_id);
        debug_assert!(self.focus.1.is_some())
    }

    /// Remove the focused leaf from the tree, otherwise panic
    pub(crate) fn remove(&mut self) -> Option<Node<T>> {
        #[cfg(not(test))]
        debug_assert!(self.pending_update.is_empty());
        let (tree_id, leaf_id) = self.focus;
        let leaf_id = leaf_id?;

        let mut next_focus = self.neighbour(&leaf_id, Direction::Before);

        let tree = self.get_tree(&tree_id).clone();
        let mut tree = tree.borrow_mut();
        let remove_idx = tree.child_index(&leaf_id);
        tree.children.remove(remove_idx);
        let removed = self.nodes.remove(&leaf_id);
        debug_assert!(removed.is_some());

        if let Some(parent_id) = tree.parent {
            let parent = self.get_tree(&parent_id);
            let mut parent = parent.borrow_mut();

            // Cleanup empty tree
            let empty_tree = tree.children.is_empty() && tree.id != self.root;
            let single_child = tree.children.len() == 1 && tree.id != self.root;
            let no_leaf_in_tree = !parent.has_leaf() && tree.id != self.root;

            if empty_tree {
                let idx = parent.child_index(&tree_id);
                parent.children.remove(idx);
                next_focus = (parent.id, parent.children.last().cloned());
                drop(parent);
                drop(tree);
                self.update_geometries(&parent_id);
                self.nodes.remove(&tree_id);
            } else if single_child || no_leaf_in_tree {
                let children: Vec<_> = tree.children.drain(..).collect();
                for id in &children {
                    self.nodes.get(id).unwrap().set_parent_id(&parent_id)
                }
                let idx = parent.child_index(&tree_id);
                parent.children.remove(idx);
                parent.children.extend(children);
                next_focus = (parent.id, parent.children.last().cloned());
                drop(parent);
                drop(tree);

                self.update_geometries(&parent_id);
                self.nodes.remove(&tree_id);
            } else {
                drop(tree);
                drop(parent);
                self.update_geometries(&tree_id);
            }
        } else {
            drop(tree);
            self.update_geometries(&tree_id);
        }

        if self.focus == next_focus {
            self.focus = (self.root, None)
        } else {
            self.focus = next_focus;
        }

        removed
    }

    // Walk up the tree from the given id until a leaf is find before or after this node
    // Returns both the found leaf and its parent node
    //                      0    <-    3. nothing was found on the previous step, repeat starting from node(2)
    //                     / \
    //                    1   4  <-    2. walk up the tree and try to find a child at node index - 1
    //                       / \
    //                     (2)  3 <-  1. starting from node(3) with `Direction::Before`
    fn neighbour(&self, id: &NodeId, direction: Direction) -> (NodeId, Option<NodeId>) {
        let parent = self
            .nodes
            .get(id)
            .and_then(|node| node.parent_id())
            .map(|id| self.get_tree(&id));

        match parent {
            None => (self.root, self.descendant_leaf(&self.root, direction)),
            Some(parent) => {
                let parent = parent.borrow();
                let idx = parent.child_index(id);

                match direction {
                    Direction::Before => {
                        if idx == 0 {
                            self.neighbour(&parent.id, direction)
                        } else {
                            let neighbour = parent.children.get(idx - 1);
                            let parent_id = parent.id;
                            self.neighbour_or_descendant(direction, neighbour, &parent_id)
                        }
                    }
                    Direction::After => {
                        if idx == parent.children.len() - 1 {
                            self.neighbour(&parent.id, direction)
                        } else {
                            let neighbour = parent.children.get(idx + 1);
                            let parent_id = parent.id;
                            self.neighbour_or_descendant(direction, neighbour, &parent_id)
                        }
                    }
                }
            }
        }
    }

    fn neighbour_or_descendant(
        &self,
        direction: Direction,
        neighbour: Option<&NodeId>,
        parent_id: &NodeId,
    ) -> (NodeId, Option<NodeId>) {
        match neighbour {
            Some(NodeId::Leaf(_)) => (*parent_id, neighbour.copied()),
            _ => self
                .descendant_leaf(parent_id, direction.invert())
                .map(|id| (self.get_leaf(&id).borrow().parent.unwrap(), Some(id)))
                .unwrap_or((
                    self.root,
                    self.descendant_leaf(&self.root, direction.invert()),
                )),
        }
    }

    fn update_geometries(&mut self, tree_id: &NodeId) {
        let tree = self.get_tree(tree_id);
        let tree = tree.clone();
        let tree = tree.borrow();

        if tree.children.is_empty() {
            return;
        }

        let mut next_loc = tree.geometry.loc;
        let mut default_ratio_count = 0;
        let mut total_non_default_ratio = 0.0;

        for child_id in &tree.children {
            let node = self.nodes.get(child_id).unwrap();
            match node.ratio() {
                None => default_ratio_count += 1,
                Some(ratio) => total_non_default_ratio += ratio,
            };
        }

        let default_ratio = if default_ratio_count == 0 {
            0.0
        } else {
            (1.0 - total_non_default_ratio) / default_ratio_count as f32
        };

        for child in &tree.children {
            self.pending_update.push(*child);
            let node = self.nodes.get(child).expect("child not found");
            let ratio = node.ratio().unwrap_or(default_ratio);
            let (width, height) = match tree.orientation {
                Orientation::Vertical => (
                    tree.geometry.size.w,
                    (tree.geometry.size.h as f32 * ratio) as i32,
                ),
                Orientation::Horizontal => (
                    (tree.geometry.size.w as f32 * ratio) as i32,
                    tree.geometry.size.h,
                ),
            };

            let geometry = Rectangle::from_loc_and_size(next_loc, (width, height));

            next_loc = match tree.orientation {
                Orientation::Vertical => (geometry.loc.x, geometry.loc.y + geometry.size.h).into(),
                Orientation::Horizontal => {
                    (geometry.loc.x + geometry.size.w, geometry.loc.y).into()
                }
            };
            node.set_geometry(geometry);

            if let Node::Tree(_) = node {
                self.update_geometries(child);
            };
        }
    }

    fn descendant_leaf(&self, node_id: &NodeId, direction: Direction) -> Option<NodeId> {
        debug_assert!(matches!(node_id, NodeId::Tree(_)));
        let tree = self.get_tree(node_id);
        let tree = tree.borrow();
        let descendant = match direction {
            Direction::Before => tree.children.first(),
            Direction::After => tree.children.last(),
        };

        match descendant {
            None => None,
            Some(NodeId::Leaf(_)) => descendant.cloned(),
            Some(id) => self.descendant_leaf(id, direction),
        }
    }

    /// Return a Tree node by id, panics if the id point to a non-tree node
    fn get_tree(&self, id: &NodeId) -> &Rc<RefCell<TreeNode>> {
        debug_assert!(matches!(id, NodeId::Tree(_)));
        let Some(Node::Tree(tree)) = self.nodes.get(id) else {
            unreachable!("invalid tree");
        };

        tree
    }

    /// Return a Leaf node by id, None if the id point to a non-leaf node
    fn get_leaf(&self, id: &NodeId) -> &Rc<RefCell<Leaf<T>>> {
        debug_assert!(matches!(id, NodeId::Leaf(_)));
        let Some(Node::Leaf(leaf)) = self.nodes.get(id) else {
            unreachable!("invalid leaf");
        };

        leaf
    }

    fn get_root(&self) -> &Rc<RefCell<TreeNode>> {
        let Some(Node::Tree(tree)) = self.nodes.get(&self.root) else {
            unreachable!("root not should be set");
        };

        tree
    }
}

#[cfg(test)]
mod test {
    use sealed_test::prelude::*;
    use smithay::utils::Rectangle;

    use crate::shell::node::NodeId;
    use crate::shell::{Direction, Orientation, Tree};

    #[sealed_test]
    fn should_insert_in_root() {
        //    1
        //  /   \
        // 2     3
        let mut tree = Tree::new(Rectangle::default(), Orientation::Horizontal);
        tree.insert(());
        tree.get_pending_updates();
        tree.insert(());
        tree.get_pending_updates();

        let root = tree.get_root();
        let root = root.borrow();

        assert_eq!(root.children, [NodeId::Leaf(2), NodeId::Leaf(3)]);
        assert!(tree.nodes.get(&NodeId::Leaf(2)).is_some());
        assert!(tree.nodes.get(&NodeId::Leaf(3)).is_some());
    }

    #[sealed_test]
    fn should_insert_remove_in_root() {
        //    1
        //  / | \
        // 2  3  4
        let mut tree = Tree::new(Rectangle::default(), Orientation::Horizontal);

        tree.insert(());
        tree.get_pending_updates();
        tree.insert(());
        tree.get_pending_updates();
        tree.insert(());
        tree.get_pending_updates();

        tree.remove();

        //    1
        //  /   \
        // 2     3
        let root = tree.get_root();
        let root = root.borrow();

        assert_eq!(root.children, [NodeId::Leaf(2), NodeId::Leaf(3)]);
        assert!(tree.nodes.get(&NodeId::Tree(1)).is_some());
        assert!(tree.nodes.get(&NodeId::Leaf(2)).is_some());
        assert!(tree.nodes.get(&NodeId::Leaf(3)).is_some());
    }

    #[sealed_test]
    fn should_split_insert() {
        //   1
        //  / \
        // 2   4
        //    / \
        //   3   5
        let mut tree = Tree::new(Rectangle::default(), Orientation::Horizontal);

        tree.insert(());
        tree.get_pending_updates();
        tree.insert(());
        tree.get_pending_updates();
        tree.split_insert((), Orientation::Vertical);
        tree.get_pending_updates();

        let root = tree.get_root();
        let root = root.borrow();

        assert_eq!(root.children, [NodeId::Leaf(2), NodeId::Tree(4)]);

        let subtree = tree.get_tree(&NodeId::Tree(4));
        let subtree = subtree.borrow();

        assert_eq!(subtree.children, [NodeId::Leaf(3), NodeId::Leaf(5)]);

        assert!(tree.nodes.get(&NodeId::Tree(1)).is_some());
        assert!(tree.nodes.get(&NodeId::Leaf(2)).is_some());
        assert!(tree.nodes.get(&NodeId::Leaf(3)).is_some());
        assert!(tree.nodes.get(&NodeId::Tree(4)).is_some());
        assert!(tree.nodes.get(&NodeId::Leaf(5)).is_some());
    }

    #[sealed_test]
    fn should_get_neighbours() {
        //   1
        //  / \
        // 2   4
        //    / \
        //   3   5
        let mut tree = Tree::new(Rectangle::default(), Orientation::Horizontal);

        tree.insert(());
        tree.get_pending_updates();
        tree.insert(());
        tree.get_pending_updates();
        tree.split_insert((), Orientation::Vertical);
        tree.get_pending_updates();
        tree.get_pending_updates();

        assert_eq!(
            (NodeId::Tree(4), Some(NodeId::Leaf(3))),
            tree.neighbour(&NodeId::Leaf(5), Direction::Before)
        );
        assert_eq!(
            (NodeId::Tree(1), Some(NodeId::Leaf(2))),
            tree.neighbour(&NodeId::Leaf(3), Direction::Before)
        );
        assert_eq!(
            (NodeId::Tree(1), None),
            tree.neighbour(&NodeId::Leaf(2), Direction::Before)
        );
    }

    #[sealed_test]
    fn should_clean_up_empty_nodes() {
        //   1
        //  / \
        // 2   4
        //    / \
        //   3   5
        let mut tree = Tree::new(Rectangle::default(), Orientation::Horizontal);

        tree.insert(());
        tree.get_pending_updates();

        tree.insert(());
        tree.get_pending_updates();

        tree.split_insert((), Orientation::Vertical);
        tree.get_pending_updates();

        tree.remove();
        tree.get_pending_updates();

        tree.remove();
        tree.get_pending_updates();

        //   1
        //   |
        //   2
        let root = tree.get_root();
        let root = root.borrow();

        assert_eq!(root.children, [NodeId::Leaf(2)]);
        assert!(tree.nodes.get(&NodeId::Leaf(3)).is_none());
        assert!(tree.nodes.get(&NodeId::Tree(4)).is_none());
        assert!(tree.nodes.get(&NodeId::Leaf(5)).is_none());
    }

    #[sealed_test]
    fn should_update_geometries() {
        //   1
        //  / \
        // 2   4
        //   /   \
        //  3     5
        let mut tree = Tree::new(
            Rectangle::from_loc_and_size((0, 0), (100, 200)),
            Orientation::Horizontal,
        );

        tree.insert(());
        assert_eq!(tree.pending_update, [NodeId::Leaf(2)]);
        tree.get_pending_updates();

        let node = tree.nodes.get(&NodeId::Tree(1)).unwrap();
        let leaf = tree.nodes.get(&NodeId::Leaf(2)).unwrap();
        assert_eq!(
            node.geometry(),
            Rectangle::from_loc_and_size((0, 0), (100, 200))
        );
        assert_eq!(
            leaf.geometry(),
            Rectangle::from_loc_and_size((0, 0), (100, 200))
        );

        tree.insert(());
        assert_eq!(tree.pending_update, [NodeId::Leaf(2), NodeId::Leaf(3)]);
        tree.get_pending_updates();

        let node = tree.nodes.get(&NodeId::Tree(1)).unwrap();
        let leaf1 = tree.nodes.get(&NodeId::Leaf(2)).unwrap();
        let leaf2 = tree.nodes.get(&NodeId::Leaf(3)).unwrap();
        assert_eq!(
            node.geometry(),
            Rectangle::from_loc_and_size((0, 0), (100, 200))
        );
        assert_eq!(
            leaf1.geometry(),
            Rectangle::from_loc_and_size((0, 0), (50, 200))
        );
        assert_eq!(
            leaf2.geometry(),
            Rectangle::from_loc_and_size((50, 0), (50, 200))
        );

        tree.split_insert((), Orientation::Vertical);
        assert_eq!(
            tree.pending_update,
            [
                NodeId::Leaf(2),
                NodeId::Tree(4),
                NodeId::Leaf(3),
                NodeId::Leaf(5)
            ]
        );
        tree.get_pending_updates();

        let node = tree.nodes.get(&NodeId::Tree(1)).unwrap();
        let leaf1 = tree.nodes.get(&NodeId::Leaf(2)).unwrap();
        let leaf2 = tree.nodes.get(&NodeId::Leaf(3)).unwrap();
        let node2 = tree.nodes.get(&NodeId::Tree(4)).unwrap();
        let leaf3 = tree.nodes.get(&NodeId::Leaf(5)).unwrap();
        assert_eq!(
            node.geometry(),
            Rectangle::from_loc_and_size((0, 0), (100, 200))
        );
        assert_eq!(
            leaf1.geometry(),
            Rectangle::from_loc_and_size((0, 0), (50, 200))
        );
        assert_eq!(
            node2.geometry(),
            Rectangle::from_loc_and_size((50, 0), (50, 200))
        );
        assert_eq!(
            leaf2.geometry(),
            Rectangle::from_loc_and_size((50, 0), (50, 100))
        );
        assert_eq!(
            leaf3.geometry(),
            Rectangle::from_loc_and_size((50, 100), (50, 100))
        );
    }

    #[sealed_test]
    fn should_update_focus_on_removal() {
        let mut tree = Tree::new(Default::default(), Orientation::Horizontal);

        tree.insert(());
        tree.get_pending_updates();

        tree.insert(());
        tree.get_pending_updates();

        tree.split_insert((), Orientation::Vertical);
        tree.get_pending_updates();

        tree.set_focus((NodeId::Tree(1), NodeId::Leaf(2)));
        tree.get_pending_updates();

        tree.split_insert((), Orientation::Vertical);
        tree.get_pending_updates();

        tree.remove();
        tree.get_pending_updates();

        tree.remove();
        tree.get_pending_updates();
    }
}
