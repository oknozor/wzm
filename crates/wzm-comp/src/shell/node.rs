use crate::shell::container::ContainerRef;
use crate::shell::windows::WzmWindow;

#[derive(Debug, Clone)]
pub enum Node {
    Container(ContainerRef),
    Window(WzmWindow),
}

#[derive(Debug, Default, Clone)]
pub struct NodeEdge {
    pub left: Option<Node>,
    pub right: Option<Node>,
    pub up: Option<Node>,
    pub down: Option<Node>,
}

impl Node {
    pub fn is_container(&self) -> bool {
        matches!(self, Node::Container(_))
    }

    pub fn is_window(&self) -> bool {
        matches!(self, Node::Window(_))
    }

    pub fn id(&self) -> u32 {
        match self {
            Node::Container(container) => container.get().id,
            Node::Window(w) => w.id(),
        }
    }
}

impl TryInto<WzmWindow> for Node {
    // TODO: this error
    type Error = &'static str;

    fn try_into(self) -> Result<WzmWindow, Self::Error> {
        match self {
            Node::Window(w) => Ok(w),
            _ => Err("tried to unwrap a window got a container or a x11 popup"),
        }
    }
}

impl<'a> TryInto<&'a mut WzmWindow> for &'a mut Node {
    // TODO: this error
    type Error = &'static str;

    fn try_into(self) -> Result<&'a mut WzmWindow, Self::Error> {
        match self {
            Node::Window(w) => Ok(w),
            _ => Err("tried to unwrap a window got a container or a x11 popup"),
        }
    }
}

impl TryInto<WzmWindow> for &Node {
    // TODO: this error
    type Error = &'static str;

    fn try_into(self) -> Result<WzmWindow, Self::Error> {
        match self {
            Node::Window(w) => Ok(w.clone()),
            _ => Err("tried to unwrap a window got a container or a x11 popup"),
        }
    }
}

impl TryInto<ContainerRef> for Node {
    // TODO: this error
    type Error = &'static str;

    fn try_into(self) -> Result<ContainerRef, Self::Error> {
        match self {
            Node::Container(c) => Ok(c),
            _ => Err("tried to unwrap a container got a window"),
        }
    }
}

impl TryInto<ContainerRef> for &Node {
    // TODO: this error
    type Error = &'static str;

    fn try_into(self) -> Result<ContainerRef, Self::Error> {
        match self {
            Node::Container(c) => Ok(c.clone()),
            _ => Err("tried to unwrap a container got a window"),
        }
    }
}

impl<'a> TryInto<&'a WzmWindow> for &'a Node {
    // TODO: this error
    type Error = &'static str;

    fn try_into(self) -> Result<&'a WzmWindow, Self::Error> {
        match self {
            Node::Window(w) => Ok(w),
            _ => Err("tried to unwrap a window got a container or a x11 popup"),
        }
    }
}

impl<'a> TryInto<&'a ContainerRef> for &'a Node {
    // TODO: this error
    type Error = &'static str;

    fn try_into(self) -> Result<&'a ContainerRef, Self::Error> {
        match self {
            Node::Container(c) => Ok(c),
            _ => Err("tried to unwrap a container got a window"),
        }
    }
}

pub mod id {
    use once_cell::sync::Lazy;
    use std::sync::{Arc, Mutex};

    static NODE_ID_COUNTER: Lazy<Arc<Mutex<u32>>> = Lazy::new(|| Arc::new(Mutex::new(0)));

    pub fn get() -> u32 {
        let id = NODE_ID_COUNTER.lock().unwrap();
        *id
    }

    pub fn next() -> u32 {
        let mut id = NODE_ID_COUNTER.lock().unwrap();
        *id += 1;
        *id
    }
}
