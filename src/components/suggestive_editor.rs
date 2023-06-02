use std::{cell::RefCell, rc::Rc};

use crossterm::event::{Event, KeyCode};
use lsp_types::CompletionItem;

use crate::{buffer::Buffer, screen::Dispatch};

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
}

impl DropdownItem for CompletionItem {
    fn label(&self) -> String {
        self.label.clone()
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
        let cursor_point = self.editor().get_cursor_point();
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
                        // TODO: should use edit if available
                        self.editor.replace_previous_word(&completion.label);
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
                (event, _) => {
                    let dispatches = self.editor.handle_event(state, event)?;
                    if let Some(dropdown) = &self.dropdown {
                        dropdown
                            .borrow_mut()
                            .set_filter(&self.editor.get_current_word());
                    }

                    Ok(dispatches
                        .into_iter()
                        .chain(match self.editor().buffer().path() {
                            None => vec![],
                            Some(path) => vec![Dispatch::RequestCompletion {
                                component_id: self.id(),
                                path,
                                position: lsp_types::Position {
                                    line: cursor_point.row as u32,
                                    character: cursor_point.column as u32,
                                },
                            }],
                        })
                        .collect())
                }
            }
        } else {
            match event {
                Event::Key(key) if key.code == KeyCode::Char('1') => {
                    match self.editor().buffer().path() {
                        None => Ok(vec![]),
                        Some(path) => Ok(vec![Dispatch::RequestHover {
                            component_id: self.id(),
                            path,
                            position: lsp_types::Position {
                                line: cursor_point.row as u32,
                                character: cursor_point.column as u32,
                            },
                        }]),
                    }
                }
                _ => Ok(self.editor.handle_event(state, event)?),
            }
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
        }
    }

    pub fn set_completion(&mut self, dropdown_items: Vec<CompletionItem>) {
        if let Some(dropdown) = &self.dropdown {
            dropdown.borrow_mut().set_items(dropdown_items);
        } else {
            self.dropdown = Some(Rc::new(RefCell::new(Dropdown::new(DropdownConfig {
                title: "Completion".to_string(),
                items: dropdown_items,
            }))));
        }
    }

    pub fn set_hover(&mut self, hover: lsp_types::Hover) {
        fn marked_string_to_string(marked_string: lsp_types::MarkedString) -> String {
            match marked_string {
                lsp_types::MarkedString::String(string) => string,
                lsp_types::MarkedString::LanguageString(language_string) => language_string.value,
            }
        }
        let content = match hover.contents {
            lsp_types::HoverContents::Scalar(marked_string) => {
                marked_string_to_string(marked_string)
            }
            lsp_types::HoverContents::Array(contents) => contents
                .into_iter()
                .map(marked_string_to_string)
                .collect::<Vec<_>>()
                .join("----------------\n\n"),
            lsp_types::HoverContents::Markup(content) => content.value,
        };
        self.set_info("Hover", content)
    }

    fn show_documentation(&mut self, completion: Option<CompletionItem>) {
        if let Some(completion) = completion {
            self.set_info(
                "Documentation",
                completion
                    .documentation
                    .map(|doc| match doc {
                        lsp_types::Documentation::String(s) => s,
                        lsp_types::Documentation::MarkupContent(content) => content.value,
                    })
                    .unwrap_or_default(),
            )
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
