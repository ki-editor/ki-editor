use crate::app::Dispatch;
use crate::context::Context;
use crate::grid::StyleKey;
use crate::lsp::code_action::CodeAction;
use crate::lsp::completion::CompletionItemEdit;
use crate::lsp::signature_help::SignatureHelp;

use crate::selection_range::SelectionRange;
use crate::{
    buffer::Buffer,
    lsp::completion::{Completion, CompletionItem},
};

use itertools::Itertools;
use my_proc_macros::key;
use shared::icons::get_icon_config;
use std::{cell::RefCell, rc::Rc};

use super::component::ComponentId;
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
    info_panel: Option<Rc<RefCell<Editor>>>,

    code_action_dropdown: Dropdown<CodeAction>,
    completion_dropdown: Dropdown<CompletionItem>,

    trigger_characters: Vec<String>,
    filter: SuggestiveEditorFilter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SuggestiveEditorFilter {
    CurrentWord,
    CurrentLine,
}

impl DropdownItem for CodeAction {
    fn label(&self) -> String {
        self.title()
    }
    fn info(&self) -> Option<Info> {
        None
    }

    fn group() -> Option<Box<dyn Fn(&Self) -> String>> {
        Some(Box::new(|item| {
            item.kind.clone().unwrap_or("Unknown".to_string())
        }))
    }
}

impl DropdownItem for CompletionItem {
    fn emoji(&self) -> String {
        self.kind
            .map(|kind| {
                get_icon_config()
                    .completion
                    .get(&format!("{:?}", kind))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("({:?})", kind))
            })
            .unwrap_or_default()
    }
    fn label(&self) -> String {
        self.label()
    }
    fn info(&self) -> Option<Info> {
        let kind = self.kind.map(|kind| {
            convert_case::Casing::to_case(&format!("{:?}", kind), convert_case::Case::Title)
        });
        let detail = self.detail.clone();

        let documentation = self.documentation().map(|d| d.content);
        Some(Info::new(
            [].into_iter()
                .chain(kind)
                .chain(detail)
                .chain(documentation)
                .collect_vec()
                .join("\n==========\n"),
        ))
    }

    fn group() -> Option<Box<dyn Fn(&Self) -> String>> {
        None
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
    ) -> anyhow::Result<Vec<Dispatch>> {
        if self.editor.mode == Mode::Insert && event == key!("esc") {
            self.close_all_subcomponents();
            self.editor.enter_normal_mode()?;
            return Ok(vec![]);
        }
        if self.editor.mode == Mode::Insert && self.completion_dropdown_opened() {
            match event {
                key!("ctrl+n") | key!("down") => {
                    self.completion_dropdown.next_item();
                    return Ok([Dispatch::RenderDropdown {
                        owner_id: self.id(),
                        render: self.completion_dropdown.render(),
                    }]
                    .to_vec());
                }
                key!("ctrl+p") | key!("up") => {
                    self.completion_dropdown.previous_item();
                    return Ok([Dispatch::RenderDropdown {
                        owner_id: self.id(),
                        render: self.completion_dropdown.render(),
                    }]
                    .to_vec());
                }
                key!("tab") => {
                    let current_item = self.completion_dropdown.current_item();
                    if let Some(completion) = current_item {
                        let edit_dispatch = match completion.edit {
                            None => Dispatch::DispatchEditor(DispatchEditor::ReplacePreviousWord(
                                completion.label(),
                            )),
                            Some(edit) => match edit {
                                CompletionItemEdit::PositionalEdit(edit) => {
                                    Dispatch::DispatchEditor(DispatchEditor::ApplyPositionalEdit(
                                        edit,
                                    ))
                                }
                            },
                        };
                        return Ok([
                            Dispatch::CloseDropdown {
                                owner_id: self.id(),
                            },
                            edit_dispatch,
                        ]
                        .to_vec());
                    }
                }
                _ => {}
            }
        }
        if self.editor.mode == Mode::Normal && self.code_action_dropdown.current_item().is_some() {
            match event {
                key!("ctrl+n") | key!("down") => {
                    self.code_action_dropdown.next_item();
                    return Ok([Dispatch::RenderDropdown {
                        owner_id: self.id(),
                        render: self.completion_dropdown.render(),
                    }]
                    .to_vec());
                }
                key!("ctrl+p") | key!("up") => {
                    self.code_action_dropdown.previous_item();
                    return Ok([Dispatch::RenderDropdown {
                        owner_id: self.id(),
                        render: self.completion_dropdown.render(),
                    }]
                    .to_vec());
                }
                key!("enter") => {
                    let current_item = self.code_action_dropdown.current_item();
                    if let Some(code_action) = current_item {
                        let params = self.editor.get_request_params();
                        let dispatches = code_action
                            .edit
                            .map(Dispatch::ApplyWorkspaceEdit)
                            .into_iter()
                            // A command this code action executes. If a code action
                            // provides an edit and a command, first the edit is
                            // executed and then the command.
                            // Refer https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#codeAction
                            .chain(params.and_then(|params| {
                                code_action
                                    .command
                                    .map(|command| Dispatch::LspExecuteCommand { command, params })
                            }))
                            .collect_vec();
                        return Ok(dispatches
                            .into_iter()
                            .chain(Some(Dispatch::CloseDropdown {
                                owner_id: self.id(),
                            }))
                            .collect_vec());
                        // self.menu_opened = false;
                        // self.info_panel = None;
                    }
                }
                key!("esc") => {
                    return Ok([Dispatch::CloseDropdown {
                        owner_id: self.id(),
                    }]
                    .to_vec());
                }

                _ => {}
            }
        };
        // Every other character typed in Insert mode should update the dropdown to show
        // relevant completions.
        let dispatches = self.editor.handle_key_event(context, event)?;

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

        self.code_action_dropdown.set_filter(&filter);
        self.completion_dropdown.set_filter(&filter);
        let dropdown_render = if self.completion_dropdown_opened() {
            Some(self.completion_dropdown.render())
        } else if self.code_action_dropdown.current_item().is_some() {
            Some(self.code_action_dropdown.render())
        } else {
            None
        };
        let dispatches = dispatches
            .into_iter()
            .chain(dropdown_render.map(|render| Dispatch::RenderDropdown {
                render,
                owner_id: self.id(),
            }))
            .chain(if self.editor.mode == Mode::Insert {
                self.editor
                    .get_request_params()
                    .map(|params| {
                        vec![
                            Dispatch::RequestCompletion(params.clone()),
                            Dispatch::RequestSignatureHelp(params),
                        ]
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            })
            .collect::<Vec<_>>();

        Ok(dispatches)
    }

    fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
        vec![self
            .info_panel
            .clone()
            .map(|info_panel| info_panel as Rc<RefCell<dyn Component>>)]
    }

    fn remove_child(&mut self, component_id: ComponentId) {
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
            completion_dropdown: Dropdown::new(DropdownConfig {
                title: "Completion".to_string(),
            }),
            code_action_dropdown: Dropdown::new(DropdownConfig {
                title: "Code Actions".to_string(),
            }),
            trigger_characters: vec![],
            filter,
        }
    }

    pub fn show_signature_help(&mut self, signature_help: Option<SignatureHelp>) {
        if self.editor.mode != Mode::Insert {
            return;
        }
        if let Some(info) = signature_help.and_then(|s| s.into_info()) {
            self.show_info("Signature help", info);
        } else {
            self.info_panel = None;
        }
    }

    pub fn show_info(&mut self, title: &str, info: Info) {
        let mut editor = Editor::from_text(tree_sitter_md::language(), &info.content);
        editor.set_decorations(info.decorations());
        editor.set_title(title.into());
        self.info_panel = Some(Rc::new(RefCell::new(editor)));
    }

    pub fn handle_dispatch(
        &mut self,
        dispatch: DispatchSuggestiveEditor,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match dispatch {
            DispatchSuggestiveEditor::SetCompletionFilter(filter) => {
                self.filter = filter;
                Ok([].to_vec())
            }
            DispatchSuggestiveEditor::SetCompletion(completion) => {
                if self.editor.mode == Mode::Insert {
                    self.set_completion(completion);
                    Ok([Dispatch::RenderDropdown {
                        owner_id: self.id(),
                        render: self.completion_dropdown.render(),
                    }]
                    .to_vec())
                } else {
                    Ok(Vec::new())
                }
            }
            DispatchSuggestiveEditor::SetCodeActions(code_actions) => {
                if self.editor.mode != Mode::Normal || code_actions.is_empty() {
                    return Ok(Vec::new());
                }
                self.code_action_dropdown.set_items(code_actions);

                Ok([Dispatch::RenderDropdown {
                    owner_id: self.id(),
                    render: self.code_action_dropdown.render(),
                }]
                .to_vec())
            }
        }
    }

    pub fn enter_insert_mode(&mut self) -> Result<(), anyhow::Error> {
        self.editor
            .enter_insert_mode(super::editor::Direction::Start)
    }

    pub fn completion_dropdown_current_item(&mut self) -> Option<CompletionItem> {
        self.completion_dropdown.current_item()
    }

    pub fn completion_dropdown_opened(&self) -> bool {
        self.completion_dropdown.current_item().is_some()
    }

    #[cfg(test)]
    pub fn filtered_dropdown_items(&self) -> Vec<String> {
        todo!("remove this method")
    }

    fn close_all_subcomponents(&mut self) {
        self.info_panel = None;
    }

    pub fn set_completion(&mut self, completion: Completion) {
        self.completion_dropdown.set_items(completion.items);
        self.trigger_characters = completion.trigger_characters;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchSuggestiveEditor {
    SetCompletionFilter(SuggestiveEditorFilter),
    SetCompletion(Completion),
    SetCodeActions(Vec<CodeAction>),
}

#[cfg(test)]
mod test_suggestive_editor {
    use crate::components::editor::DispatchEditor::*;
    use crate::components::suggestive_editor::DispatchSuggestiveEditor::*;
    use crate::lsp::code_action::CodeAction;
    use crate::lsp::completion::{CompletionItemEdit, PositionalEdit};
    use crate::lsp::workspace_edit::{TextDocumentEdit, WorkspaceEdit};
    use crate::position::Position;
    use crate::{
        app::Dispatch,
        buffer::Buffer,
        components::{component::Component, editor::Direction},
        lsp::completion::{Completion, CompletionItem},
        test_app::test_app::execute_test,
        test_app::test_app::ExpectKind::*,
        test_app::test_app::Step::*,
    };
    use lsp_types::CompletionItemKind;
    use my_proc_macros::{key, keys};
    use shared::canonicalized_path::CanonicalizedPath;
    use std::{cell::RefCell, rc::Rc};
    use Dispatch::*;

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
        editor.handle_events(keys!("p a"))?;

        // Expect the completion dropdown to be open,
        // and the dropdown items to be filtered
        assert!(editor.completion_dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), vec!["Patrick"]);

        // Type in space, then 's'
        editor.handle_events(keys!("space s"))?;

        // Expect the completion dropdown to be hidden,
        // and the dropdown items to be filtered by the current line, 'pa s'
        assert!(!editor.completion_dropdown_opened());
        assert_eq!(editor.filtered_dropdown_items(), Vec::new() as Vec<String>);

        // Type in enter
        editor.handle_events(keys!("tab"))?;

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
        editor.handle_events(keys!("ctrl+e"))?;

        // Enter a next line
        editor.handle_events(keys!("enter h e l l o"))?;

        // Expect the content to be updated
        assert_eq!(editor.editor().text(), "pa s\n\nhello");

        // Expect the current line is 'hello'
        assert_eq!(editor.editor().current_line()?, "hello");

        // Go to the previous line
        editor.handle_events(keys!("esc l p p p"))?;

        // Expect the current line is empty
        assert_eq!(editor.editor().current_line()?, "");

        // Type in 's'
        editor.editor_mut().enter_insert_mode(Direction::Start)?;
        editor.handle_events(keys!("s"))?;

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
            .into_iter()
            .any(|dispatch| matches!(&dispatch, Dispatch::RequestCompletion(_))));

        // Enter normal mode
        editor.handle_events(keys!("esc")).unwrap();

        // Type something
        let dispatches = editor.handle_events(keys!("l")).unwrap();

        // Expect the completion request to not be sent
        assert!(!dispatches
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
                SuggestiveEditor(SetCompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(SetCompletion(dummy_completion())),
                // Expect the completion dropdown to be open,
                Expect(CompletionDropdownContent("Patrick\nSpongebob\nSquidward")),
                // Type in 'pa'
                App(HandleKeyEvents(keys!("p a").to_vec())),
                // Expect the dropdown items to be filtered
                Expect(CompletionDropdownIsOpen(true)),
                Expect(CompletionDropdownContent("Patrick")),
                // Press tab
                App(HandleKeyEvent(key!("tab"))),
                // Expect the buffer to contain the selected item
                Expect(CurrentComponentContent("Patrick")),
                Expect(CompletionDropdownIsOpen(false)),
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
                SuggestiveEditor(SetCompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                App(HandleKeyEvents(keys!("s p o n g e").to_vec())),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(SetCompletion(Completion {
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
                    }],
                })),
                App(HandleKeyEvent(key!("tab"))),
                Expect(CurrentComponentContent("Spongebob")),
                App(HandleKeyEvents(keys!("e n d").to_vec())),
                Expect(CurrentComponentContent("Spongebobend")),
            ])
        })
    }

    #[test]
    fn code_action() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("a.to_s".to_string())),
                SuggestiveEditor(SetCompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                SuggestiveEditor(SetCodeActions(
                    [CodeAction {
                        title: "".to_string(),
                        kind: None,
                        edit: Some(WorkspaceEdit {
                            edits: [TextDocumentEdit {
                                path: s.main_rs(),
                                edits: [PositionalEdit {
                                    range: Position::new(0, 2)..Position::new(0, 6),
                                    new_text: "to_string".to_string(),
                                }]
                                .to_vec(),
                            }]
                            .to_vec(),
                            resource_operations: Vec::new(),
                        }),
                        command: None,
                    }]
                    .to_vec(),
                )),
                App(HandleKeyEvent(key!("enter"))),
                Expect(CurrentComponentContent("a.to_string")),
            ])
        })
    }

    #[test]
    fn navigate_dropdown() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(SetCompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(SetCompletion(dummy_completion())),
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
                SuggestiveEditor(SetCompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(SetCompletion(dummy_completion())),
                App(HandleKeyEvents(keys!("p a").to_vec())),
                Expect(CompletionDropdownIsOpen(true)),
                Expect(CompletionDropdownContent("Patrick")),
                // Type in one of the trigger characters, '.'
                App(HandleKeyEvent(key!("."))),
                Expect(CompletionDropdownIsOpen(true)),
                // Expect dropdown items to be unfiltered (showing all items)
                Expect(CompletionDropdownContent("Patrick\nSpongebob\nSquidward")),
            ])
        })
    }

    #[test]
    fn filter_with_current_word() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(SetCompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                SuggestiveEditor(SetCompletion(dummy_completion())),
                App(HandleKeyEvents(keys!("p a").to_vec())),
                Expect(CompletionDropdownIsOpen(true)),
                Expect(CompletionDropdownContent("Patrick")),
                App(HandleKeyEvents(keys!("space s").to_vec())),
                // Expect the completion dropdown to be open,
                // and the dropdown items to be filtered by the current word, 's'
                Expect(CompletionDropdownContent("Spongebob\nSquidward")),
            ])
        })
    }

    #[test]
    fn setting_completion_when_not_in_insert_mode() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                Editor(SetContent("".to_string())),
                SuggestiveEditor(SetCompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterNormalMode),
                // Pretend that the LSP server returned a completion
                SuggestiveEditor(SetCompletion(dummy_completion())),
                // Expect the completion dropdown to not be opened,
                // since the editor is not in insert mode
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
                SuggestiveEditor(SetCompletionFilter(SuggestiveEditorFilter::CurrentWord)),
                Editor(EnterInsertMode(Direction::Start)),
                // Pretend that the LSP server returned a completion
                // That is without edit, but contains `kind`, which means it has emoji
                SuggestiveEditor(SetCompletion(Completion {
                    items: [CompletionItem {
                        label: "Spongebob".to_string(),
                        edit: None,
                        documentation: None,
                        sort_text: None,
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: None,
                    }]
                    .to_vec(),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Info {
    content: String,
    decorations: Vec<Decoration>,
}
impl Info {
    pub(crate) fn new(content: String) -> Info {
        Info {
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

    pub fn take(self) -> (String, Vec<Decoration>) {
        let Self {
            content,
            decorations,
        } = self;
        (content, decorations)
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
            content,
            decorations,
        }
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
