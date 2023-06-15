use std::{cell::RefCell, rc::Rc};

use crossterm::event::{Event, KeyCode};

use crate::{
    buffer::Buffer,
    lsp::completion::{Completion, CompletionItem},
    position::Position,
    screen::Dispatch,
};

use super::{
    component::Component,
    dropdown::{Dropdown, DropdownConfig, DropdownItem},
    editor::{Editor, Mode},
};

/// Editor with auto-complete
pub struct SuggestiveEditor {
    editor: Editor,
    dropdown: Option<Rc<RefCell<Dropdown<CompletionItem>>>>,
    info: Option<Rc<RefCell<Editor>>>,
    trigger_characters: Vec<String>,
}

impl DropdownItem for CompletionItem {
    fn label(&self) -> String {
        self.label()
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
                (Event::Key(key), Some(dropdown)) if key.code == KeyCode::Down => {
                    let completion = dropdown.borrow_mut().next_item();
                    self.show_documentation(completion);
                    Ok(vec![])
                }
                (Event::Key(key), Some(dropdown)) if key.code == KeyCode::Up => {
                    let completion = dropdown.borrow_mut().previous_item();
                    self.show_documentation(completion);
                    Ok(vec![])
                }
                (Event::Key(key), Some(dropdown))
                    if key.code == KeyCode::Enter && dropdown.borrow().current_item().is_some() =>
                {
                    if let Some(completion) = dropdown.borrow().current_item() {
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
                    self.info = None;
                    Ok(vec![])
                }
                (Event::Key(key), Some(_)) if key.code == KeyCode::Esc => {
                    self.dropdown = None;
                    self.info = None;
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
                            Some(path) => vec![Dispatch::RequestCompletion {
                                component_id: self.id(),
                                path,
                                position: cursor_position,
                            }],
                        })
                        .collect())
                }
            }
        } else {
            self.editor.handle_event(state, event)
        }
    }

    fn children(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        self.get_children(vec![
            self.dropdown
                .clone()
                .map(|dropdown| dropdown as Rc<RefCell<dyn Component>>),
            self.info
                .clone()
                .map(|info| info as Rc<RefCell<dyn Component>>),
        ])
    }
}

impl SuggestiveEditor {
    pub fn from_buffer(buffer: Rc<RefCell<Buffer>>) -> Self {
        Self {
            editor: Editor::from_buffer(buffer),
            dropdown: None,
            info: None,
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

    fn show_documentation(&mut self, completion: Option<CompletionItem>) {
        if let Some(completion) = completion {
            if !completion.documentation().is_empty() {
                self.set_info("Documentation", completion.documentation())
            }
        }
    }

    fn set_info(&mut self, title: &str, content: String) {
        let mut editor = Editor::from_buffer(Rc::new(RefCell::new(Buffer::new(
            tree_sitter_md::language(),
            &content,
        ))));
        editor.set_title(title.to_string());
        self.info = Some(Rc::new(RefCell::new(editor)))
    }
}
