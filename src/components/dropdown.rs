use crate::components::component::Component;
use crate::screen::State;

use super::editor::Editor;

#[derive(Clone)]
pub struct Dropdown {
    editor: Editor,
}

impl Component for Dropdown {
    fn child(&self) -> &dyn Component {
        unimplemented!()
    }

    fn child_mut(&mut self) -> &mut dyn Component {
        unimplemented!()
    }

    fn handle_event(
        &mut self,
        state: &State,
        event: crossterm::event::Event,
    ) -> Vec<crate::screen::Dispatch> {
        unimplemented!()
    }
}
