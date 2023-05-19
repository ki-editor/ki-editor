use std::any::Any;

use crossterm::event::Event;
use tree_sitter::Point;

use crate::{
    auto_key_map::Incrementable,
    grid::Grid,
    rectangle::Rectangle,
    screen::{Dimension, Dispatch, State},
};

use super::editor::Editor;

// dyn_clone::clone_trait_object!(Component);

pub trait Component: Any + AnyComponent {
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;
    fn get_grid(&self) -> Grid {
        self.editor().get_grid()
    }
    fn handle_event(&mut self, state: &State, event: Event) -> anyhow::Result<Vec<Dispatch>>;

    /// This is used for closing components that are the slaves of this component.
    fn slave_ids(&self) -> Vec<ComponentId>;

    fn get_cursor_point(&self) -> Point {
        self.editor().get_cursor_point()
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

    fn update(&mut self, str: &str) {
        self.editor_mut().update(str);
    }

    fn title(&self) -> &str {
        self.editor().title()
    }

    fn set_title(&mut self, title: String) {
        self.editor_mut().set_title(title);
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

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Copy, Hash, Default)]
pub struct ComponentId(pub usize);

impl Incrementable for ComponentId {
    fn increment(&self) -> Self {
        ComponentId(self.0 + 1)
    }
}
