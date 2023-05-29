use std::{cell::RefCell, rc::Rc};

use crossterm::event::{Event, KeyCode};

use crate::{buffer::Buffer, screen::Dispatch};

use super::{
    component::Component,
    dropdown::{Dropdown, DropdownItem},
    editor::{Editor, Mode},
};

/// Editor with auto-complete
pub struct SuggestiveEditor {
    editor: Editor,
    dropdown: Rc<RefCell<Dropdown>>,
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
    ) -> anyhow::Result<Vec<Dispatch>> {
        let cursor_point = self.editor().get_cursor_point();
        if self.editor.mode == Mode::Insert {
            match event {
                Event::Key(key) if key.code == KeyCode::Down => {
                    let completion = self.dropdown.borrow_mut().next_item();
                    self.editor.replace_previous_word(&completion);
                    Ok(vec![])
                }
                Event::Key(key) if key.code == KeyCode::Up => {
                    let completion = self.dropdown.borrow_mut().previous_item();
                    self.editor.replace_previous_word(&completion);
                    Ok(vec![])
                }
                _ => {
                    let dispatches = self.editor.handle_event(state, event)?;
                    self.dropdown
                        .borrow_mut()
                        .set_filter(&self.editor.get_current_word());
                    Ok(dispatches
                        .into_iter()
                        .chain(
                            vec![Dispatch::RequestCompletion {
                                position: lsp_types::Position {
                                    line: cursor_point.row as u32,
                                    character: cursor_point.column as u32,
                                },
                            }]
                            .into_iter(),
                        )
                        .collect())
                }
            }
        } else {
            Ok(self.editor.handle_event(state, event)?)
        }
    }

    fn slave_ids(&self) -> Vec<super::component::ComponentId> {
        todo!()
    }
}

impl SuggestiveEditor {
    pub fn from_buffer(buffer: Rc<RefCell<Buffer>>, dropdown: Rc<RefCell<Dropdown>>) -> Self {
        Self {
            editor: Editor::from_buffer(buffer),
            dropdown,
        }
    }

    pub fn set_items(&self, items: Vec<DropdownItem>) {
        self.dropdown.borrow_mut().set_items(items);
    }
}
