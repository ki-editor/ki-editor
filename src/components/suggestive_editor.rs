use crate::screen::Dispatch;
use crate::screen::RequestParams;
use crate::{
    buffer::Buffer,
    lsp::completion::{Completion, CompletionItem},
};
use crossterm::event::KeyModifiers;
use crossterm::event::{Event, KeyCode};
use std::{cell::RefCell, rc::Rc};

use super::{
    component::Component,
    dropdown::{Dropdown, DropdownConfig, DropdownItem},
    editor::{Editor, Mode},
};

/// Editor with auto-complete
pub struct SuggestiveEditor {
    editor: Editor,
    dropdown: Option<Rc<RefCell<Dropdown<CompletionItem>>>>,
    trigger_characters: Vec<String>,
}

impl DropdownItem for CompletionItem {
    fn label(&self) -> String {
        self.label()
    }
    fn info(&self) -> Option<String> {
        self.documentation()
    }
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
        let cursor_position = self.editor().get_cursor_position();
        if self.editor.mode == Mode::Insert {
            match (event, &self.dropdown) {
                (Event::Key(key), Some(dropdown))
                    if key.code == KeyCode::Down
                        || (key.modifiers == KeyModifiers::CONTROL
                            && key.code == KeyCode::Char('n')) =>
                {
                    dropdown.borrow_mut().next_item();
                    Ok(vec![])
                }
                (Event::Key(key), Some(dropdown))
                    if key.code == KeyCode::Up
                        || (key.modifiers == KeyModifiers::CONTROL
                            && key.code == KeyCode::Char('p')) =>
                {
                    dropdown.borrow_mut().previous_item();
                    Ok(vec![])
                }
                (Event::Key(key), Some(dropdown))
                    if key.code == KeyCode::Enter
                        && dropdown.borrow_mut().current_item().is_some() =>
                {
                    if let Some(completion) = dropdown.borrow_mut().current_item() {
                        match completion.edit {
                            None => {
                                self.editor.replace_previous_word(&completion.label());
                            }
                            Some(edit) => {
                                self.editor.apply_positional_edit(edit);
                            }
                        }
                    }
                    self.dropdown = None;
                    Ok(vec![])
                }
                (Event::Key(key), Some(_)) if key.code == KeyCode::Esc => {
                    self.dropdown = None;
                    self.editor.enter_normal_mode();
                    Ok(vec![])
                }

                // Every other character typed in Insert mode should update the dropdown to show
                // relevant completions.
                (event, _) => {
                    let dispatches = self.editor.handle_event(state, event)?;
                    if let Some(dropdown) = &self.dropdown {
                        let filter = {
                            // We need to subtract 1 because we need to get the character
                            // before the cursor, not the character at the cursor
                            let cursor_position = self.editor().get_cursor_position().sub_column(1);

                            match self.editor().buffer().get_char_at_position(cursor_position) {
                                // The filter should be empty if the current character is a trigger
                                // character, so that we can show all the completion items.
                                Some(current_char)
                                    if self
                                        .trigger_characters
                                        .contains(&current_char.to_string()) =>
                                {
                                    "".to_string()
                                }

                                // If the current character is not a trigger character, we should
                                // filter based on the current word under the cursor.
                                _ => self.editor.get_current_word(),
                            }
                        };

                        dropdown.borrow_mut().set_filter(&filter);
                    }

                    Ok(dispatches
                        .into_iter()
                        .chain(match self.editor().buffer().path() {
                            None => vec![],
                            Some(path) => vec![Dispatch::RequestCompletion(RequestParams {
                                component_id: self.id(),
                                path,
                                position: cursor_position,
                            })],
                        })
                        .collect())
                }
            }
        } else {
            self.editor.handle_event(state, event)
        }
    }

    fn children(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        self.get_children(vec![self
            .dropdown
            .clone()
            .map(|dropdown| dropdown as Rc<RefCell<dyn Component>>)])
    }
}

impl SuggestiveEditor {
    pub fn from_buffer(buffer: Rc<RefCell<Buffer>>) -> Self {
        Self {
            editor: Editor::from_buffer(buffer),
            dropdown: None,
            trigger_characters: vec![],
        }
    }

    pub fn set_completion(&mut self, completion: Completion) {
        let dropdown = match &self.dropdown {
            Some(dropdown) => dropdown.clone(),
            None => {
                let dropdown = Rc::new(RefCell::new(Dropdown::new(DropdownConfig {
                    title: "Completion".to_string(),
                })));
                self.dropdown = Some(dropdown.clone());
                dropdown
            }
        };

        dropdown.borrow_mut().set_items(completion.items);
        self.trigger_characters = completion.trigger_characters;
    }

    pub fn enter_insert_mode(&mut self) {
        self.editor.enter_insert_mode()
    }

    pub fn current_item(&mut self) -> Option<CompletionItem> {
        self.dropdown
            .as_ref()
            .and_then(|dropdown| dropdown.borrow_mut().current_item())
    }

    pub fn dropdown_opened(&self) -> bool {
        self.dropdown.is_some()
    }
}
