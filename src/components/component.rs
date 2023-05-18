use std::any::Any;

use crossterm::event::Event;
use tree_sitter::Point;

use crate::{
    grid::Grid,
    screen::{Dimension, Dispatch, State},
};

dyn_clone::clone_trait_object!(Component);

pub trait Component: dyn_clone::DynClone + Any + AnyComponent {
    fn child(&self) -> &dyn Component;
    fn child_mut(&mut self) -> &mut dyn Component;
    fn get_grid(&self) -> Grid {
        self.child().get_grid()
    }
    fn handle_event(&mut self, state: &State, event: Event) -> Vec<Dispatch>;

    fn get_cursor_point(&self) -> Point {
        self.child().get_cursor_point()
    }

    fn scroll_offset(&self) -> u16 {
        self.child().scroll_offset()
    }

    fn set_dimension(&mut self, dimension: Dimension) {
        self.child_mut().set_dimension(dimension)
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
