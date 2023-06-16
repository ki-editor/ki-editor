use std::{any::Any, cell::RefCell, rc::Rc};

use crossterm::event::Event;

use crate::{
    grid::Grid,
    lsp::diagnostic::Diagnostic,
    position::Position,
    rectangle::Rectangle,
    screen::{Dispatch, State},
};

use super::editor::Editor;

// dyn_clone::clone_trait_object!(Component);

pub trait Component: Any + AnyComponent {
    fn id(&self) -> ComponentId {
        self.editor().id()
    }
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;
    fn get_grid(&self, diagnostics: &[Diagnostic]) -> Grid {
        self.editor().get_grid(diagnostics)
    }
    fn handle_event(&mut self, state: &State, event: Event) -> anyhow::Result<Vec<Dispatch>>;

    fn get_cursor_position(&self) -> Position {
        self.editor().get_cursor_position()
    }

    fn scroll_offset(&self) -> u16 {
        self.editor().scroll_offset()
    }

    fn set_rectangle(&mut self, rectangle: Rectangle) {
        self.editor_mut().set_rectangle(rectangle)
    }

    fn rectangle(&self) -> &Rectangle {
        self.editor().rectangle()
    }

    fn set_content(&mut self, str: &str) {
        self.editor_mut().set_content(str);
    }

    fn title(&self) -> String {
        self.editor().title()
    }

    fn set_title(&mut self, title: String) {
        self.editor_mut().set_title(title);
    }

    fn children(&self) -> Vec<Rc<RefCell<dyn Component>>>;

    /// Helper function to get children from a vector of Option<Rc<RefCell<dyn Component>>>
    fn get_children(
        &self,
        components: Vec<Option<Rc<RefCell<dyn Component>>>>,
    ) -> Vec<Rc<RefCell<dyn Component>>> {
        components
            .into_iter()
            .flatten()
            .flat_map(|component| {
                component
                    .clone()
                    .borrow()
                    .children()
                    .into_iter()
                    .chain(std::iter::once(component))
            })
            .collect()
    }
}

/// Modified from https://github.com/helix-editor/helix/blob/91da0dc172dde1a972be7708188a134db70562c3/helix-term/src/compositor.rs#L212
pub trait AnyComponent {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> AnyComponent for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Why do I use UUID instead of a simple u64?
/// Because with UUID I don't need a global state to keep track of the next ID.
#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Copy, Hash, Default)]
pub struct ComponentId(uuid::Uuid);
impl ComponentId {
    pub fn new() -> ComponentId {
        ComponentId(uuid::Uuid::new_v4())
    }
}
