use crossterm::event::{Event, KeyCode};

use crate::{
    component::Component,
    engine::{Dispatch, Editor, HandleEventResult},
    screen::State,
};

#[derive(Clone)]
pub struct Prompt {
    editor: Editor,
    change_focused_to: usize,
}

impl Prompt {
    pub fn new(change_focused_to: usize) -> Self {
        let mut editor = Editor::from_text(tree_sitter_md::language(), "");
        editor.enter_insert_mode();
        Prompt {
            editor,
            change_focused_to,
        }
    }
}

impl Component for Prompt {
    fn child(&self) -> &dyn Component {
        &self.editor
    }
    fn child_mut(&mut self) -> &mut dyn Component {
        &mut self.editor
    }
    fn intercept_event(&mut self, _: &State, event: Event) -> HandleEventResult {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Esc => HandleEventResult::Handled(vec![Dispatch::CloseCurrentWindow {
                    change_focused_to: self.change_focused_to,
                }]),
                KeyCode::Enter => HandleEventResult::Handled(vec![
                    Dispatch::SetSearch {
                        search: self.editor.get_line(),
                    },
                    Dispatch::CloseCurrentWindow {
                        change_focused_to: self.change_focused_to,
                    },
                ]),
                _ => HandleEventResult::Ignored(Event::Key(key_event)),
            },
            _ => HandleEventResult::Ignored(event),
        }
    }
}
