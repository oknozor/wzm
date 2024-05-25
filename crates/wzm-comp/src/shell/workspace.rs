use crate::decoration::{BorderShader, CustomRenderElements};
use crate::shell::container::{Container, ContainerLayout, ContainerRef};
use crate::shell::node;
use crate::shell::node::Node;
use crate::shell::nodemap::NodeMap;
use crate::shell::windows::WindowWrap;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::{Renderer, Texture};
use smithay::desktop::{Space, Window};
use smithay::output::Output;
use smithay::utils::{Logical, Physical, Rectangle};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct WorkspaceRef {
    inner: Rc<RefCell<Workspace>>,
}

impl WorkspaceRef {
    pub fn new(output: Output, space: &Space<Window>, gaps: i32) -> Self {
        let geometry = space.output_geometry(&output).unwrap();
        Self {
            inner: Rc::new(RefCell::new(Workspace::new(&output, geometry, gaps))),
        }
    }

    pub fn get_mut(&self) -> RefMut<'_, Workspace> {
        self.inner.borrow_mut()
    }

    pub fn get(&self) -> Ref<'_, Workspace> {
        self.inner.borrow()
    }
}

impl WorkspaceRef {
    pub fn render_elements(
        &self,
        renderer: &mut GlesRenderer,
    ) -> Vec<CustomRenderElements<GlesRenderer>> {
        let mut render_elements: Vec<CustomRenderElements<_>> = Vec::new();
        let focus_id = self.get().get_focus().1.map(|w| w.id());

        for element in self.get().flatten_containers() {
            let container = element.get();

            let mut size = container.size;
            size.w += 10;
            size.h += 10;
            let mut loc = container.location;
            loc.x -= 5;
            loc.y -= 5;

            render_elements.push(CustomRenderElements::Shader(BorderShader::element(
                renderer,
                size,
                loc,
                [0.0, 0.0, 5.0],
                [0.4, 0.0, 0.0],
                None,
            )));
        }

        for element in &self.get().flatten_window() {
            let (start_color, end_color) = focus_id
                .map(|id| {
                    if id == element.id() {
                        ([0.5, 0.0, 0.0], [0.0, 0.0, 0.5])
                    } else {
                        ([0.5, 0.5, 0.0], [0.8, 0.0, 0.5])
                    }
                })
                .unwrap_or(([0.5, 0.5, 0.0], [0.8, 0.0, 0.5]));

            let window = element.inner();

            render_elements.push(CustomRenderElements::Shader(BorderShader::element(
                renderer,
                window.geometry().size,
                element.loc(),
                start_color,
                end_color,
                Some(window.clone()),
            )));
        }

        render_elements
    }
}

#[derive(Debug)]
pub struct Workspace {
    pub output: Output,
    pub fullscreen_layer: Option<Node>,
    root: ContainerRef,
    focus: ContainerRef,
    pub needs_redraw: bool,
    pub gaps: i32,
}

impl Workspace {
    pub fn new(output: &Output, geometry: Rectangle<i32, Logical>, gaps: i32) -> Workspace {
        let root = Container {
            id: node::id::get(),
            location: (geometry.loc.x + gaps, geometry.loc.y + gaps).into(),
            size: (geometry.size.w - 2 * gaps, geometry.size.h - 2 * gaps).into(),
            output: output.clone(),
            parent: None,
            nodes: NodeMap::default(),
            layout: ContainerLayout::Horizontal,
            gaps,
        };

        let root = ContainerRef::new(root);
        let focus = root.clone();

        Self {
            output: output.clone(),
            root,
            focus,
            fullscreen_layer: None,
            needs_redraw: false,
            gaps,
        }
    }

    pub fn update_layout(&mut self, space: &Space<Window>) {
        let geometry = space.output_geometry(&self.output).unwrap();
        let root = &self.root;
        let mut root = root.get_mut();
        self.needs_redraw = root.update_layout(geometry);
    }

    pub fn redraw(&mut self, space: &mut Space<Window>) {
        let geometry = space.output_geometry(&self.output).expect("Geometry");
        self.unmap_all(space);

        if let Some(layer) = &self.fullscreen_layer {
            match layer {
                Node::Container(container) => {
                    debug!("Redraw: FullScreen Container");
                    let container = container.get();
                    container.redraw(space);
                }
                Node::Window(window) => {
                    debug!("Redraw: FullScreen Window");
                    window.set_fullscreen(geometry);
                    window.map(space, true);
                }
            }
        } else {
            debug!("Redraw: Root Container");
            let root = self.root.get();
            root.redraw(space);
        }

        self.needs_redraw = false;
    }

    pub fn root(&self) -> ContainerRef {
        self.root.clone()
    }

    pub fn get_focus(&self) -> (ContainerRef, Option<WindowWrap>) {
        // FIXME: panic here some time
        let window = {
            let c = self.focus.get();
            c.get_focused_window()
        };

        (self.focus.clone(), window)
    }

    pub fn create_container(&mut self, layout: ContainerLayout) -> ContainerRef {
        let child = {
            let (container, _) = self.get_focus();
            let parent = container.clone();
            let mut current = container.get_mut();
            current.create_child(layout, parent, self.gaps)
        };

        self.focus = child.clone();
        child
    }

    pub fn pop_container(&mut self) {
        let current = self.get_focus();
        let current = current.0.get();
        let id = current.id;
        if let Some(parent) = &current.parent {
            self.focus = parent.clone();
            let mut parent = parent.get_mut();
            parent.nodes.remove(&id);
        }
    }

    pub fn set_container_focused(&mut self, container: &ContainerRef) {
        self.focus = container.clone();
    }

    pub fn set_container_and_window_focus(
        &mut self,
        container: &ContainerRef,
        window: &WindowWrap,
    ) {
        self.focus = container.clone();
        container.get_mut().set_focus(window.id());
    }

    pub fn flatten_window(&self) -> Vec<WindowWrap> {
        let root = self.root.get();
        let mut windows: Vec<WindowWrap> = root.nodes.iter_windows().cloned().collect();

        for child in root.nodes.iter_containers() {
            let window = child.get().flatten_window();
            windows.extend_from_slice(window.as_slice());
        }

        windows
    }

    pub fn flatten_containers(&self) -> impl Iterator<Item = ContainerRef> {
        self.root
            .childs_containers()
            .into_iter()
            .flat_map(|container| container.childs_containers())
    }

    pub fn unmap_all(&mut self, space: &mut Space<Window>) {
        for window in self.flatten_window() {
            space.unmap_elem(window.inner());
        }
    }

    pub fn find_container_by_id(&self, id: &u32) -> Option<ContainerRef> {
        if &self.root.get().id == id {
            Some(self.root.clone())
        } else {
            self.root.find_container_by_id(id)
        }
    }

    pub fn reset_gaps(&self, space: &Space<Window>) {
        let geometry = space
            .output_geometry(&self.output)
            .expect("Output should have a geometry");
        let mut container = self.root.get_mut();
        container.location = (geometry.loc.x + self.gaps, geometry.loc.y + self.gaps).into();
        container.size = (
            geometry.size.w - 2 * self.gaps,
            geometry.size.h - 2 * self.gaps,
        )
            .into();
    }

    pub fn get_output_geometry_f64(
        &self,
        space: &Space<Window>,
    ) -> Option<Rectangle<f64, Physical>> {
        space.output_geometry(&self.output).map(|geometry| {
            let scale = self.output.current_scale().fractional_scale();
            geometry.to_f64().to_physical_precise_up(scale)
        })
    }
}
