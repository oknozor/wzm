use smithay::utils::{Logical, Rectangle};
use crate::shell2::node::NodeId;

pub struct Leaf<T> {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub geometry: Rectangle<i32, Logical>,
    pub ratio: Option<f32>,
    pub data: T,
}
