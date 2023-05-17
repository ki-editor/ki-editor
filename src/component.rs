use crossterm::event::Event;
use tree_sitter::Point;

use crate::{
    engine::{Direction, Dispatch, HandleEventResult},
    grid::Grid,
    screen::{Dimension, State},
    selection::SelectionMode,
};

dyn_clone::clone_trait_object!(Component);

pub trait Component: dyn_clone::DynClone {
    fn child(&self) -> &dyn Component;
    fn child_mut(&mut self) -> &mut dyn Component;
    fn intercept_event(&mut self, state: &State, event: Event) -> HandleEventResult;
    fn get_grid(&self) -> Grid {
        self.child().get_grid()
    }

    /// Normally, this method does not needs to be implemented,
    /// because we are taking the bubble-down approach instead of the bubble-up approach.
    ///
    /// However, if you want to allow the bottom most component to handle unintercepted events,
    /// you can override this method.
    fn handle_event(&mut self, state: &State, event: Event) -> Vec<Dispatch> {
        let result = self.intercept_event(state, event);
        match result {
            HandleEventResult::Handled(dispatches) => dispatches,
            HandleEventResult::Ignored(event) => self.child_mut().handle_event(state, event),
            HandleEventResult::Teed { dispatches, event } => dispatches
                .into_iter()
                .chain(self.child_mut().handle_event(state, event).into_iter())
                .collect(),
        }
    }

    fn get_cursor_point(&self) -> Point {
        self.child().get_cursor_point()
    }
    fn scroll_offset(&self) -> u16 {
        self.child().scroll_offset()
    }
    fn set_dimension(&mut self, dimension: Dimension) {
        self.child_mut().set_dimension(dimension)
    }
    fn select(&mut self, selection_mode: SelectionMode, direction: Direction) {
        self.child_mut().select(selection_mode, direction)
    }
}
