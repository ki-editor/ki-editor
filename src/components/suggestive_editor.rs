use crate::context::Context;
use crate::screen::Dispatch;
use crate::screen::RequestParams;
use crate::{
    buffer::Buffer,
    lsp::completion::{Completion, CompletionItem},
};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use std::{cell::RefCell, rc::Rc};

use super::component::ComponentId;
use super::{
    component::Component,
    dropdown::{Dropdown, DropdownConfig, DropdownItem},
    editor::{Editor, Mode},
};

/// Editor with auto-complete
pub struct SuggestiveEditor {
    editor: Editor,
    info_panel: Option<Rc<RefCell<Editor>>>,
    dropdown: Option<Rc<RefCell<Dropdown<CompletionItem>>>>,
    trigger_characters: Vec<String>,
    filter: SuggestiveEditorFilter,
}

pub enum SuggestiveEditorFilter {
    CurrentWord,
    CurrentLine,
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
        context: &mut Context,
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
                    let dispatches = self.editor.handle_event(context, event)?;
                    if let Some(dropdown) = &self.dropdown {
                        let filter = match self.filter {
                            SuggestiveEditorFilter::CurrentWord => {
                                // We need to subtract 1 because we need to get the character
                                // before the cursor, not the character at the cursor
                                let cursor_position =
                                    self.editor().get_cursor_position().sub_column(1);

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
                            }
                            SuggestiveEditorFilter::CurrentLine => {
                                let buffer = self.editor().buffer();
                                buffer
                                    .get_line(self.get_cursor_position().to_char_index(&buffer))
                                    .to_string()
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
            self.editor.handle_event(context, event)
        }
    }

    fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
        vec![
            self.dropdown.clone().and_then(|dropdown| {
                if dropdown.borrow().filtered_items().is_empty() {
                    None
                } else {
                    Some(dropdown as Rc<RefCell<dyn Component>>)
                }
            }),
            self.info_panel
                .clone()
                .map(|info_panel| info_panel as Rc<RefCell<dyn Component>>),
        ]
    }

    fn remove_child(&mut self, component_id: ComponentId) {
        if matches!(&self.dropdown, Some(dropdown) if dropdown.borrow().id() == component_id) {
            self.dropdown = None;
        }
        if matches!(&self.info_panel, Some(info_panel) if info_panel.borrow().id() == component_id)
        {
            self.info_panel = None;
        }
    }
}

impl SuggestiveEditor {
    pub fn from_buffer(buffer: Rc<RefCell<Buffer>>, filter: SuggestiveEditorFilter) -> Self {
        Self {
            editor: Editor::from_buffer(buffer),
            info_panel: None,
            dropdown: None,
            trigger_characters: vec![],
            filter,
        }
    }

    pub fn show_info(&mut self, info: String) {
        self.info_panel = Some(Rc::new(RefCell::new(Editor::from_text(
            tree_sitter_md::language(),
            &info,
        ))));
    }

    pub fn set_completion(&mut self, completion: Completion) {
        if self.editor.mode != Mode::Insert {
            return;
        }
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
        self.descendants().iter().any(|descendant| {
            descendant
                .borrow()
                .as_any()
                .downcast_ref::<Dropdown<CompletionItem>>()
                .is_some()
        })
    }

    #[cfg(test)]
    pub fn filtered_dropdown_items(&self) -> Vec<String> {
        self.dropdown
            .as_ref()
            .map(|dropdown| {
                dropdown
                    .borrow()
                    .filtered_items()
                    .iter()
                    .map(|item| item.label())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod test_suggestive_editor {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        buffer::Buffer,
        components::component::Component,
        lsp::completion::{Completion, CompletionItem, PositionalEdit},
        position::Position,
    };

    use super::{SuggestiveEditor, SuggestiveEditorFilter};
    use pretty_assertions::assert_eq;

    fn dummy_completion() -> Completion {
        Completion {
            trigger_characters: vec![".".to_string()],
            items: vec![
                CompletionItem::from_label("Spongebob".to_string()),
                CompletionItem::from_label("Patrick".to_string()),
                CompletionItem::from_label("Squidward".to_string()),
            ],
        }
    }

    fn editor(filter: SuggestiveEditorFilter) -> SuggestiveEditor {
        SuggestiveEditor::from_buffer(
            Rc::new(RefCell::new(Buffer::new(tree_sitter_md::language(), ""))),
            filter,
        )
    }

    #[test]
    fn navigate_dropdown() {
        let mut editor = editor(SuggestiveEditorFilter::CurrentWord);

        // Enter insert mode
        editor.handle_events("i").unwrap();

        // Pretend that the LSP server returned a completion
        editor.set_completion(dummy_completion());

        // Expect the completion dropdown to be open
        assert!(editor.dropdown_opened());

        // Expect the completion dropdown to show all the items
        assert_eq!(
            editor.filtered_dropdown_items(),
            vec!["Patrick", "Spongebob", "Squidward"]
        );

        // Expect the selected item to be the first item
        assert_eq!(
            editor.current_item(),
            Some(CompletionItem::from_label("Patrick".to_string()))
        );

        // Go to the next item using the down arrow key
        editor.handle_events("down").unwrap();

        // Expect the selected item to be the second item
        assert_eq!(
            editor.current_item(),
            Some(CompletionItem::from_label("Spongebob".to_string()))
        );

        // Go to the previous item using the up arrow key
        editor.handle_events("up").unwrap();

        // Expect the selected item to be the first item
        assert_eq!(
            editor.current_item(),
            Some(CompletionItem::from_label("Patrick".to_string()))
        );

        // Go to the next item using Ctrl-n
        editor.handle_events("c-n").unwrap();

        // Expect the selected item to be the second item
        assert_eq!(
            editor.current_item(),
            Some(CompletionItem::from_label("Spongebob".to_string()))
        );

        // Go to the previous item using Ctrl-p
        editor.handle_events("c-p").unwrap();

        // Expect the selected item to be the first item
        assert_eq!(
            editor.current_item(),
            Some(CompletionItem::from_label("Patrick".to_string()))
        );
    }

    #[test]
    fn trigger_characters() {
        let mut editor = editor(SuggestiveEditorFilter::CurrentWord);

        // Enter insert mode
        editor.handle_events("i").unwrap();

        // Pretend that the LSP server returned a completion
        editor.set_completion(dummy_completion());

        // Type in 'pa'
        editor.handle_events("p a").unwrap();

        // Expect the completion dropdown to be open,
        // and the dropdown items to be filtered
        assert!(editor.dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), vec!["Patrick"]);

        // Type in one of the trigger characters, '.'
        editor.handle_events(".").unwrap();

        // Expect the completion dropdown to be open,
        // and the dropdown items to be unfiltered (showing all items)
        assert!(editor.dropdown_opened());

        // Expect the completion dropdown to show all the items
        assert_eq!(
            editor.filtered_dropdown_items(),
            vec!["Patrick", "Spongebob", "Squidward"]
        );
    }

    #[test]
    fn completion_without_edit() {
        let mut editor = editor(SuggestiveEditorFilter::CurrentWord);

        // Enter insert mode
        editor.handle_events("i").unwrap();

        // Pretend that the LSP server returned a completion
        editor.set_completion(dummy_completion());

        // Type in 'pa'
        editor.handle_events("p a").unwrap();

        // Expect the completion dropdown to be open,
        // and the dropdown items to be filtered
        assert!(editor.dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), vec!["Patrick"]);

        // Press enter
        editor.handle_events("enter").unwrap();

        // Expect the completion dropdown to be closed
        assert!(!editor.dropdown_opened());

        // Expect the buffer to contain the selected item
        assert_eq!(editor.editor().text(), "Patrick");
    }

    #[test]
    fn completion_with_edit() {
        let mut editor = editor(SuggestiveEditorFilter::CurrentWord);

        // Enter insert mode
        editor.handle_events("i").unwrap();

        // Enter a word 'sponge'
        editor.handle_events("s p o n g e").unwrap();

        // Pretend that the LSP server returned a completion
        editor.set_completion(Completion {
            trigger_characters: vec![".".to_string()],
            items: vec![CompletionItem {
                label: "Spongebob".to_string(),
                edit: Some(PositionalEdit {
                    range: Position::new(0, 0)..Position::new(0, 6),
                    new_text: "Spongebob".to_string(),
                }),
                documentation: None,
                sort_text: None,
            }],
        });

        // Press enter
        editor.handle_events("enter").unwrap();

        // Expect the content of the buffer to be applied with the new edit,
        // resulting in 'Spongebob'
        assert_eq!(editor.editor().text(), "Spongebob");
    }

    #[test]
    fn filter_with_current_word() {
        let mut editor = editor(SuggestiveEditorFilter::CurrentWord);

        // Enter insert mode
        editor.handle_events("i").unwrap();

        // Pretend that the LSP server returned a completion
        editor.set_completion(dummy_completion());

        // Type in 'pa'
        editor.handle_events("p a").unwrap();

        // Expect the completion dropdown to be open,
        // and the dropdown items to be filtered
        assert!(editor.dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), vec!["Patrick"]);

        // Type in space, then 's'
        editor.handle_events("space s").unwrap();

        // Expect the completion dropdown to be open,
        // and the dropdown items to be filtered by the current word, 's'
        assert_eq!(
            editor.filtered_dropdown_items(),
            vec!["Spongebob", "Squidward"]
        );
    }

    #[test]
    fn filter_with_current_line() {
        let mut editor = editor(SuggestiveEditorFilter::CurrentLine);

        // Enter insert mode
        editor.handle_events("i").unwrap();

        // Pretend that the LSP server returned a completion
        editor.set_completion(dummy_completion());

        // Type in 'pa'
        editor.handle_events("p a").unwrap();

        // Expect the completion dropdown to be open,
        // and the dropdown items to be filtered
        assert!(editor.dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), vec!["Patrick"]);

        // Type in space, then 's'
        editor.handle_events("space s").unwrap();

        // Expect the completion dropdown to be hidden,
        // and the dropdown items to be filtered by the current line, 'pa s'
        assert!(!editor.dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), Vec::new() as Vec<String>);
    }

    #[test]
    fn enter_when_no_filtered_items() {
        let mut editor = editor(SuggestiveEditorFilter::CurrentWord);

        // Enter insert mode
        editor.handle_events("i").unwrap();

        // Pretend that the LSP server returned a completion
        editor.set_completion(dummy_completion());

        // Expect the completion dropdown to be opened
        assert!(editor.dropdown_opened());

        // Type in 'x'
        editor.handle_events("x").unwrap();

        // Expect the completion dropdown to be closed,
        // since there are no filtered items
        assert!(!editor.dropdown_opened());

        // Press enter
        editor.handle_events("enter").unwrap();

        // Expect a newline to be inserted
        assert_eq!(editor.editor().text(), "x\n");
    }
}
