use crossterm::event::Event;

use crate::components::component::Component;
use crate::screen::{Dispatch, State};
use crate::selection::SelectionMode;

use super::component::ComponentId;
use super::editor::{Direction, Editor};

#[derive(Clone)]
pub struct Dropdown {
    editor: Editor,
}

impl Dropdown {
    pub fn new() -> Self {
        let mut editor = Editor::from_text(tree_sitter_md::language(), "");
        editor.select(SelectionMode::Line, Direction::Current);
        Self { editor }
    }

    pub fn next_item(&mut self) -> String {
        self.editor.select(SelectionMode::Line, Direction::Forward);
        self.editor.get_current_line().trim().to_string()
    }

    pub fn previous_item(&mut self) -> String {
        self.editor.select(SelectionMode::Line, Direction::Backward);
        self.editor.get_current_line().trim().to_string()
    }
}

impl Component for Dropdown {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_event(&mut self, state: &State, event: Event) -> Vec<Dispatch> {
        self.editor.handle_event(state, event)
    }

    fn slave_ids(&self) -> Vec<ComponentId> {
        vec![]
    }
}
