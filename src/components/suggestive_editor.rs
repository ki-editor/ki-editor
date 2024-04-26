use crate::app::{Dispatch, Dispatches};
use crate::context::Context;
use crate::grid::StyleKey;
use crate::lsp::completion::CompletionItemEdit;
use DispatchEditor::*;

use crate::selection_range::SelectionRange;
use crate::{
    buffer::Buffer,
    lsp::completion::{Completion, CompletionItem},
};

use itertools::Itertools;
use my_proc_macros::key;
use std::{cell::RefCell, rc::Rc};

use super::dropdown::{Dropdown, DropdownConfig};
use super::editor::DispatchEditor;
use super::{
    component::Component,
    dropdown::DropdownItem,
    editor::{Editor, Mode},
};

/// Editor with auto-complete
pub struct SuggestiveEditor {
    editor: Editor,
    completion_dropdown: Dropdown,

    trigger_characters: Vec<String>,
    filter: SuggestiveEditorFilter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SuggestiveEditorFilter {
    CurrentWord,
    CurrentLine,
}

impl From<CompletionItem> for DropdownItem {
    fn from(value: CompletionItem) -> Self {
        DropdownItem::new(format!("{} {}", value.emoji(), value.label()))
            .set_info(value.info())
            .set_dispatches(Dispatches::one(match value.edit {
                None => Dispatch::ToEditor(ReplacePreviousWord(value.label())),
                Some(edit) => match edit {
                    CompletionItemEdit::PositionalEdit(edit) => {
                        Dispatch::ToEditor(ApplyPositionalEdit(edit))
                    }
                },
            }))
    }
}

impl Component for SuggestiveEditor {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_key_event(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        if self.editor.mode == Mode::Insert && self.completion_dropdown_opened() {
            match event {
                key!("ctrl+n") | key!("down") => {
                    self.completion_dropdown.next_item();
                    return Ok([Dispatch::RenderDropdown {
                        render: self.completion_dropdown.render(),
                    }]
                    .to_vec()
                    .into());
                }
                key!("ctrl+p") | key!("up") => {
                    self.completion_dropdown.previous_item();
                    return Ok([Dispatch::RenderDropdown {
                        render: self.completion_dropdown.render(),
                    }]
                    .to_vec()
                    .into());
                }
                key!("tab") => {
                    let current_item = self.completion_dropdown.current_item();
                    if let Some(completion) = current_item {
                        return Ok(
                            Dispatches::one(Dispatch::CloseDropdown).chain(completion.dispatches)
                        );
                    }
                }

                _ => {}
            }
        }

        // Every other character typed in Insert mode should update the dropdown to show
        // relevant completions.
        let dispatches = self.editor.handle_key_event(context, event.clone())?;

        let filter = match self.filter {
            SuggestiveEditorFilter::CurrentWord => {
                // We need to subtract 1 because we need to get the character
                // before the cursor, not the character at the cursor
                let cursor_position = self.editor().get_cursor_position()?.sub_column(1);

                match self.editor().buffer().get_char_at_position(cursor_position) {
                    // The filter should be empty if the current character is a trigger
                    // character, so that we can show all the completion items.
                    Some(current_char)
                        if self.trigger_characters.contains(&current_char.to_string()) =>
                    {
                        "".to_string()
                    }

                    // If the current character is not a trigger character, we should
                    // filter based on the current word under the cursor.
                    _ => self.editor.get_current_word()?,
                }
            }
            SuggestiveEditorFilter::CurrentLine => self.editor().current_line()?,
        };

        self.completion_dropdown.set_filter(&filter);
        let dropdown_render = if self.completion_dropdown_opened() {
            Some(self.completion_dropdown.render())
        } else {
            None
        };
        Ok(dispatches.chain(match event {
            key!("esc") => [
                Dispatch::CloseDropdown,
                Dispatch::CloseEditorInfo,
                Dispatch::ToEditor(EnterNormalMode),
            ]
            .to_vec()
            .into(),
            _ if self.editor.mode == Mode::Insert => self
                .editor
                .get_request_params()
                .map(|params| {
                    vec![
                        Dispatch::RequestCompletion(params.clone()),
                        Dispatch::RequestSignatureHelp(params),
                    ]
                })
                .unwrap_or_default()
                .into_iter()
                .chain(dropdown_render.map(|render| Dispatch::RenderDropdown { render }))
                .collect_vec()
                .into(),
            _ => Default::default(),
        }))
    }

    fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
        Default::default()
    }
}

impl SuggestiveEditor {
    pub fn from_buffer(buffer: Rc<RefCell<Buffer>>, filter: SuggestiveEditorFilter) -> Self {
        Self {
            editor: Editor::from_buffer(buffer),
            completion_dropdown: Dropdown::new(DropdownConfig {
                title: "Completion".to_string(),
            }),
            trigger_characters: vec![],
            filter,
        }
    }

    pub fn handle_dispatch(
        &mut self,
        dispatch: DispatchSuggestiveEditor,
    ) -> anyhow::Result<Dispatches> {
        match dispatch {
            DispatchSuggestiveEditor::CompletionFilter(filter) => {
                self.filter = filter;
                Ok(Default::default())
            }
            DispatchSuggestiveEditor::Completion(completion) => {
                if self.editor.mode == Mode::Insert {
                    self.set_completion(completion);
                    Ok([Dispatch::RenderDropdown {
                        render: self.completion_dropdown.render(),
                    }]
                    .to_vec()
                    .into())
                } else {
                    Ok(Vec::new().into())
                }
            }
        }
    }

    pub fn enter_insert_mode(&mut self) -> Result<(), anyhow::Error> {
        self.editor
            .enter_insert_mode(super::editor::Direction::Start)
    }

    pub fn completion_dropdown_current_item(&mut self) -> Option<DropdownItem> {
        self.completion_dropdown.current_item()
    }

    pub fn completion_dropdown_opened(&self) -> bool {
        !self.completion_dropdown.items().is_empty()
    }

    #[cfg(test)]
    pub fn filtered_dropdown_items(&self) -> Vec<String> {
        todo!("remove this method")
    }

    pub fn set_completion(&mut self, completion: Completion) {
        self.completion_dropdown.set_items(completion.items);
        self.trigger_characters = completion.trigger_characters;
    }

    pub(crate) fn render_completion_dropdown(&self) -> Dispatches {
        [Dispatch::RenderDropdown {
            render: self.completion_dropdown.render(),
        }]
        .to_vec()
        .into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchSuggestiveEditor {
    CompletionFilter(SuggestiveEditorFilter),
    Completion(Completion),
}

#[cfg(test)]
mod test_suggestive_editor {
    use crate::components::editor::DispatchEditor::*;
    use crate::components::suggestive_editor::DispatchSuggestiveEditor::*;
    use crate::lsp::completion::{CompletionItemEdit, PositionalEdit};
    use crate::lsp::documentation::Documentation;
    use crate::position::Position;
    use crate::{
        app::Dispatch,
        buffer::Buffer,
        components::{component::Component, editor::Direction},
        lsp::completion::{Completion, CompletionItem},
        test_app::execute_test,
        test_app::ExpectKind::*,
        test_app::Step::*,
    };
    use lsp_types::CompletionItemKind;
    use my_proc_macros::{key, keys};
    use shared::canonicalized_path::CanonicalizedPath;
    use std::{cell::RefCell, rc::Rc};
    use Dispatch::*;

    use super::{Info, SuggestiveEditor, SuggestiveEditorFilter};
    use pretty_assertions::assert_eq;

    fn dummy_completion() -> Completion {
        Completion {
            trigger_characters: vec![".".to_string()],
            items: vec![
                CompletionItem::from_label("Spongebob".to_string()),
                CompletionItem::from_label("Patrick".to_string()),
                CompletionItem::from_label("Squidward".to_string()),
            ]
            .into_iter()
            .map(|item| item.into())
            .collect(),
        }
    }

    fn editor(filter: SuggestiveEditorFilter) -> SuggestiveEditor {
        SuggestiveEditor::from_buffer(
            Rc::new(RefCell::new(Buffer::new(tree_sitter_md::language(), ""))),
            filter,
        )
    }

    #[test]
    #[ignore]
    fn filter_with_current_line() -> anyhow::Result<()> {
        let mut editor = editor(SuggestiveEditorFilter::CurrentLine);

        // Enter insert mode
        editor
            .editor_mut()
            .enter_insert_mode(Direction::Start)
            .unwrap();

        // Pretend that the LSP server returned a completion
        editor.set_completion(dummy_completion());

        // Type in 'pa'
        let _ = editor.handle_events(keys!("p a"))?;

        // Expect the completion dropdown to be open,
        // and the dropdown items to be filtered
        assert!(editor.completion_dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), vec!["Patrick"]);

        // Type in space, then 's'
        let _ = editor.handle_events(keys!("space s"))?;

        // Expect the completion dropdown to be hidden,
        // and the dropdown items to be filtered by the current line, 'pa s'
        assert!(!editor.completion_dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), Vec::new() as Vec<String>);

        // Type in enter
        let _ = editor.handle_events(keys!("tab"))?;

        // Expect a new line is added
        assert_eq!(editor.editor().text(), "pa s\n");

        // Expect the current line is empty
        assert_eq!(editor.editor().current_line()?, "");

        // Expect the completion dropdown to be open,
        // and all dropdown items to be shown,
        // because the current line is empty
        assert!(editor.completion_dropdown_opened());
        assert_eq!(
            editor.filtered_dropdown_items(),
            vec!["Patrick", "Spongebob", "Squidward"]
        );

        // Close the dropdown menu
        let _ = editor.handle_events(keys!("ctrl+e"))?;

        // Enter a next line
        let _ = editor.handle_events(keys!("enter h e l l o"))?;

        // Expect the content to be updated
        assert_eq!(editor.editor().text(), "pa s\n\nhello");

        // Expect the current line is 'hello'
        assert_eq!(editor.editor().current_line()?, "hello");

        // Go to the previous line
        let _ = editor.handle_events(keys!("esc l p p p"))?;

        // Expect the current line is empty
        assert_eq!(editor.editor().current_line()?, "");

        // Type in 's'
        editor.editor_mut().enter_insert_mode(Direction::Start)?;
        let _ = editor.handle_events(keys!("s"))?;

        // Expect the current line is 's'
        assert_eq!(editor.editor().current_line()?, "s");

        assert_eq!(editor.editor().text(), "pa s\ns\nhello",);

        // Expect the completion dropdown to be open,
        // and the dropdown items to be filtered by the current line, 's'
        assert!(editor.completion_dropdown_opened());
        assert_eq!(
            editor.filtered_dropdown_items(),
            vec!["Spongebob", "Squidward"]
        );
        Ok(())
    }

    //    #[test]
    //    fn dropdown_should_be_excluded_from_descendants_by_dropdown_opened() {
    //        let mut editor = editor(SuggestiveEditorFilter::CurrentWord);
    //
    //        // Enter insert mode
    //        editor
    //            .editor_mut()
    //            .enter_insert_mode(Direction::Start)
    //            .unwrap();
    //
    //        // Pretend that the LSP server returned a completion
    //        editor.set_completion(dummy_completion());
    //
    //        // Expect the completion dropdown to be opened
    //        assert!(editor.dropdown_opened());
    //
    //        // Expect the dropdown to be included in descendants
    //        assert!(editor
    //            .descendants()
    //            .iter()
    //            .any(|d| d.borrow().id() == editor.dropdown.borrow().id()));
    //
    //        // Set the dropdown to be closed
    //        editor.dropdown_opened = false;
    //
    //        // Expect the dropdown to be excluded from descendants
    //        assert!(!editor
    //            .descendants()
    //            .iter()
    //            .any(|d| d.borrow().id() == editor.dropdown.borrow().id()));
    //    }

    #[test]
    fn typing_in_insert_mode_should_request_completion() {
        let mut editor = editor(SuggestiveEditorFilter::CurrentWord);

        let file = tempfile::NamedTempFile::new().unwrap();

        let path: CanonicalizedPath = file.path().to_path_buf().try_into().unwrap();

        editor.editor_mut().buffer_mut().set_path(path);

        // Enter insert mode
        editor
            .editor_mut()
            .enter_insert_mode(Direction::Start)
            .unwrap();

        // Type something
        let dispatches = editor.handle_events(keys!("p")).unwrap();

        // Expect the completion request to be sent
        assert!(dispatches
            .into_vec()
            .into_iter()
            .any(|dispatch| matches!(&dispatch, Dispatch::RequestCompletion(_))));
    }

    #[test]
    fn completion_without_edit() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(Completion(dummy_completion())),
                // Expect the completion dropdown to be open,
                Expect(CompletionDropdownContent(
                    " Patrick\n Spongebob\n Squidward",
                )),
                // Type in 'pa'
                App(HandleKeyEvents(keys!("p a").to_vec())),
                // Expect the dropdown items to be filtered
                Expect(CompletionDropdownIsOpen(true)),
                Expect(CompletionDropdownContent(" Patrick")),
                // Press tab
                App(HandleKeyEvent(key!("tab"))),
                // Expect the buffer to contain the selected item
                Expect(CurrentComponentContent("Patrick")),
                Expect(CompletionDropdownIsOpen(false)),
            ])
        })
    }

    #[test]
    fn completion_info_documentation() -> anyhow::Result<()> {
        let completion_item = |label: &str, documentation: Option<&str>| CompletionItem {
            label: label.to_string(),
            edit: Some(CompletionItemEdit::PositionalEdit(PositionalEdit {
                range: Position::new(0, 0)..Position::new(0, 6),
                new_text: label.to_string(),
            })),
            documentation: documentation.map(Documentation::new),
            sort_text: None,
            kind: None,
            detail: None,
        };
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(super::DispatchSuggestiveEditor::Completion(Completion {
                    trigger_characters: vec![".".to_string()],
                    items: vec![
                        completion_item("Spongebob", Some("krabby patty maker")),
                        completion_item("Zatrick Mazerick", None),
                    ]
                    .into_iter()
                    .map(|item| item.into())
                    .collect(),
                })),
                // Expect the "Completion Info" panel is shown, because "Spongebob" has doc
                Expect(AppGridContains("Completion Info")),
                Expect(AppGridContains("patty maker")),
                App(HandleKeyEvents(keys!("Z a t r i c k").to_vec())),
                Expect(AppGridContains("atrick")),
                // Expect the "Completion Info" panel is hidden, because "patrick" has no doc
                Expect(Not(Box::new(AppGridContains("Completion Info")))),
            ])
        })
    }

    #[test]
    fn completion_with_edit() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                App(HandleKeyEvents(keys!("s p o n g e").to_vec())),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(Completion(Completion {
                    trigger_characters: vec![".".to_string()],
                    items: vec![CompletionItem {
                        label: "Spongebob".to_string(),
                        edit: Some(CompletionItemEdit::PositionalEdit(PositionalEdit {
                            range: Position::new(0, 0)..Position::new(0, 6),
                            new_text: "Spongebob".to_string(),
                        })),
                        documentation: None,
                        sort_text: None,
                        kind: None,
                        detail: None,
                    }]
                    .into_iter()
                    .map(|item| item.into())
                    .collect(),
                })),
                App(HandleKeyEvent(key!("tab"))),
                Expect(CurrentComponentContent("Spongebob")),
                App(HandleKeyEvents(keys!("e n d").to_vec())),
                Expect(CurrentComponentContent("Spongebobend")),
            ])
        })
    }

    #[test]
    fn navigate_dropdown() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(Completion(dummy_completion())),
                Expect(CompletionDropdownIsOpen(true)),
                Expect(CompletionDropdownSelectedItem("Patrick")),
                App(HandleKeyEvent(key!("down"))),
                Expect(CompletionDropdownSelectedItem("Spongebob")),
                App(HandleKeyEvent(key!("up"))),
                Expect(CompletionDropdownSelectedItem("Patrick")),
                App(HandleKeyEvent(key!("ctrl+n"))),
                Expect(CompletionDropdownSelectedItem("Spongebob")),
                App(HandleKeyEvent(key!("ctrl+p"))),
                Expect(CompletionDropdownSelectedItem("Patrick")),
            ])
        })
    }

    #[test]
    fn trigger_characters() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(Completion(dummy_completion())),
                App(HandleKeyEvents(keys!("p a").to_vec())),
                Expect(CompletionDropdownIsOpen(true)),
                Expect(CompletionDropdownContent(" Patrick")),
                // Type in one of the trigger characters, '.'
                App(HandleKeyEvent(key!("."))),
                Expect(CompletionDropdownIsOpen(true)),
                // Expect dropdown items to be unfiltered (showing all items)
                Expect(CompletionDropdownContent(
                    " Patrick\n Spongebob\n Squidward",
                )),
            ])
        })
    }

    #[test]
    fn enter_normal_mode_should_close_completion_dropdown() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(Completion(dummy_completion())),
                Expect(CompletionDropdownIsOpen(true)),
                Expect(CurrentPath(s.main_rs())),
                App(HandleKeyEvent(key!("esc"))),
                Expect(CompletionDropdownIsOpen(false)),
                App(HandleKeyEvent(key!("n"))),
                Expect(CompletionDropdownIsOpen(false)),
            ])
        })
    }

    #[test]
    fn enter_normal_mode_should_close_info() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                App(ShowEditorInfo(Info::default())),
                Expect(EditorInfoOpen(true)),
                Expect(CurrentPath(s.main_rs())),
                App(HandleKeyEvent(key!("esc"))),
                Expect(EditorInfoOpen(false)),
            ])
        })
    }

    #[test]
    fn receiving_multiple_completion_should_not_increase_dropdown_infos_count(
    ) -> Result<(), anyhow::Error> {
        let completion = Completion {
            trigger_characters: vec![".".to_string()],
            items: [CompletionItem::from_label("hello".to_string())
                .set_documentation(Some(Documentation::new("This is a doc")))]
            .into_iter()
            .map(|item| item.into())
            .collect(),
        };
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                Expect(DropdownInfosCount(0)),
                SuggestiveEditor(Completion(completion.clone())),
                Expect(DropdownInfosCount(1)),
                SuggestiveEditor(Completion(completion.clone())),
                Expect(DropdownInfosCount(1)),
            ])
        })
    }

    #[test]
    fn filter_with_current_word() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(Completion(dummy_completion())),
                App(HandleKeyEvents(keys!("p a").to_vec())),
                Expect(CompletionDropdownIsOpen(true)),
                Expect(CompletionDropdownContent(" Patrick")),
                App(HandleKeyEvents(keys!("space s").to_vec())),
                // Expect the completion dropdown to be open,
                // and the dropdown items to be filtered by the current word, 's'
                Expect(CompletionDropdownContent(" Spongebob\n Squidward")),
            ])
        })
    }

    #[test]
    fn setting_completion_when_not_in_insert_mode() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterNormalMode),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(Completion(dummy_completion())),
                // Expect the completion dropdown to not be opened,
                // since the editor is not in insert mode
                Expect(CompletionDropdownIsOpen(false)),
                Editor(MoveSelection(crate::components::editor::Movement::Next)),
                Expect(CompletionDropdownIsOpen(false)),
            ])
        })
    }

    #[test]
    fn completion_with_emoji() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(CompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                // Pretend that the LSP server returned a completion
                // That is without edit, but contains `kind`, which means it has emoji
                SuggestiveEditor(Completion(Completion {
                    items: [CompletionItem {
                        label: "Spongebob".to_string(),
                        edit: None,
                        documentation: None,
                        sort_text: None,
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: None,
                    }]
                    .into_iter()
                    .map(|item| item.into())
                    .collect(),
                    trigger_characters: Vec::new(),
                })),
                App(HandleKeyEvent(key!("s"))),
                Expect(CompletionDropdownContent("ƒ Spongebob")),
                App(HandleKeyEvent(key!("tab"))),
                // Expect the content of the buffer to be applied with the new edit,
                // resulting in 'Spongebob', and does not contain emoji
                Expect(CurrentComponentContent("Spongebob")),
            ])
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Info {
    title: String,
    content: String,
    decorations: Vec<Decoration>,
}
impl Info {
    pub(crate) fn new(title: String, content: String) -> Info {
        Info {
            title,
            content,
            decorations: Vec::new(),
        }
    }

    pub fn content(&self) -> &String {
        &self.content
    }

    pub fn decorations(&self) -> &Vec<Decoration> {
        &self.decorations
    }

    pub fn take(self) -> (String, String, Vec<Decoration>) {
        let Self {
            content,
            title,
            decorations,
        } = self;
        (title, content, decorations)
    }

    pub fn set_decorations(self, decorations: Vec<Decoration>) -> Info {
        Info {
            decorations,
            ..self
        }
    }

    pub(crate) fn join(self, other: Info) -> Info {
        let separator = "=".repeat(10).to_string();
        let content = format!("{}\n{}\n{}", self.content, separator, other.content);
        let other_decorations = other
            .decorations
            .into_iter()
            .map(|decoration| decoration.increase_byte(separator.len() + 2))
            .collect_vec();
        let decorations = self
            .decorations
            .into_iter()
            .chain(other_decorations)
            .collect_vec();
        Info {
            title: self.title.clone(),
            content,
            decorations,
        }
    }

    pub(crate) fn title(&self) -> String {
        self.title.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Decoration {
    selection_range: SelectionRange,
    style_key: StyleKey,
    adjustments: Vec<Adjustment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Adjustment {
    IncreaseByte(usize),
}

impl Decoration {
    fn increase_byte(mut self, len: usize) -> Decoration {
        self.adjustments.push(Adjustment::IncreaseByte(len));
        self
    }

    pub(crate) fn selection_range(&self) -> &SelectionRange {
        &self.selection_range
    }

    pub(crate) fn style_key(&self) -> &StyleKey {
        &self.style_key
    }

    pub(crate) fn new(selection_range: SelectionRange, style_key: StyleKey) -> Decoration {
        Decoration {
            selection_range,
            style_key,
            adjustments: Default::default(),
        }
    }

    pub(crate) fn move_left(self, count: usize) -> Decoration {
        Decoration {
            selection_range: self.selection_range.move_left(count),
            ..self
        }
    }
}
