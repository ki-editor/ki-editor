use std::{cell::RefCell, rc::Rc};

use crate::{buffer::Buffer, screen::Dispatch};

use super::{
    component::Component,
    dropdown::Dropdown,
    editor::{Editor, Mode},
};

/// Editor with auto-complete
pub struct SuggestiveEditor {
    editor: Editor,
    dropdown: Option<Rc<RefCell<Dropdown>>>,
}

impl Component for SuggestiveEditor {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_event(
        &mut self,
        state: &crate::screen::State,
        event: crossterm::event::Event,
    ) -> anyhow::Result<Vec<crate::screen::Dispatch>> {
        let dispatches = self.editor.handle_event(state, event)?;
        let cursor_point = self.editor().get_cursor_point();
        if self.editor.mode == Mode::Insert {
            Ok(dispatches
                .into_iter()
                .chain(
                    vec![Dispatch::RequestSuggestions {
                        position: lsp_types::Position {
                            line: cursor_point.row as u32,
                            character: cursor_point.column as u32,
                        },
                    }]
                    .into_iter(),
                )
                .collect())
        } else {
            Ok(dispatches)
        }
    }

    fn slave_ids(&self) -> Vec<super::component::ComponentId> {
        todo!()
    }
}

impl SuggestiveEditor {
    pub fn from_buffer(buffer: Rc<RefCell<Buffer>>) -> Self {
        Self {
            editor: Editor::from_buffer(buffer),
            dropdown: None,
        }
    }
}
