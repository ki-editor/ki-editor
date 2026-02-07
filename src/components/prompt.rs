use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use itertools::Itertools;
use my_proc_macros::key;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    app::{Dispatch, DispatchParser, Dispatches},
    buffer::Buffer,
    components::{component::ComponentId, editor::DispatchEditor},
    context::Context,
    lsp::completion::Completion,
    search::parse_search_config,
    selection::SelectionMode,
    thread::Callback,
};

use super::{
    component::Component,
    dropdown::DropdownItem,
    editor::{Editor, Mode},
    editor_keymap::alted,
    suggestive_editor::{DispatchSuggestiveEditor, SuggestiveEditor, SuggestiveEditorFilter},
};

#[derive(Clone, PartialEq)]
pub enum PromptOnEnter {
    ParseCurrentLine {
        parser: DispatchParser,
        history_key: PromptHistoryKey,
        current_line: Option<String>,
        suggested_items: Vec<DropdownItem>,
    },
    ParseWholeBuffer {
        parser: DispatchParser,
        initial_lines: Vec<String>,
    },
    SelectsFirstMatchingItem {
        items: PromptItems,
    },
}

pub struct Prompt {
    editor: SuggestiveEditor,
    on_enter: PromptOnEnter,
    on_change: Option<PromptOnChangeDispatch>,
    on_cancelled: Option<Dispatches>,
    matcher: Option<PromptMatcher>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptOnChangeDispatch {
    RequestWorkspaceSymbol(CanonicalizedPath),
    SetIncrementalSearchConfig { component_id: ComponentId },
    UpdateSuggestedItemsWithChildPaths,
}

impl PromptOnChangeDispatch {
    fn to_dispatches(&self, context: &PromptContext) -> Dispatches {
        match self {
            PromptOnChangeDispatch::RequestWorkspaceSymbol(path) => {
                Dispatches::one(Dispatch::RequestWorkspaceSymbols {
                    query: context.current_line.to_string(),
                    path: path.clone(),
                })
            }
            PromptOnChangeDispatch::SetIncrementalSearchConfig { component_id } => {
                let search_config = parse_search_config(&context.current_line)
                    .unwrap_or_default()
                    .local_config;
                Dispatches::one(Dispatch::UpdateCurrentComponentTitle(format!(
                    "Local search ({})",
                    search_config.mode.display()
                )))
                .append(Dispatch::SetIncrementalSearchConfig {
                    config: search_config,
                    component_id: Some(*component_id),
                })
            }
            PromptOnChangeDispatch::UpdateSuggestedItemsWithChildPaths => {
                let path = PathBuf::from(&context.current_line);
                let path = if path.is_dir() {
                    Some(path.as_path())
                } else {
                    path.parent()
                };
                if let Some(path) = path {
                    if let Ok(paths) = get_child_directories(path) {
                        Dispatches::one(Dispatch::ToSuggestiveEditor(
                            DispatchSuggestiveEditor::Completion(Completion {
                                items: paths
                                    .into_iter()
                                    .map(|path| {
                                        DropdownItem::new(format!(
                                            "{}{}",
                                            path.display_absolute(),
                                            std::path::MAIN_SEPARATOR
                                        ))
                                    })
                                    .collect_vec(),
                                trigger_characters: Vec::new(),
                            }),
                        ))
                    } else {
                        Default::default()
                    }
                } else {
                    Default::default()
                }
            }
        }
    }
}

fn get_child_directories(path: &Path) -> anyhow::Result<Vec<CanonicalizedPath>> {
    Ok(std::fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| path.try_into().ok())
        .sorted()
        .collect())
}

struct PromptContext {
    current_line: String,
}

struct PromptMatcher {
    nucleo: nucleo::Nucleo<DropdownItem>,
}
impl PromptMatcher {
    fn reparse(&mut self, filter: &str) {
        self.nucleo
            .pattern
            .reparse(0, filter, Default::default(), Default::default(), false);
    }

    fn handle_nucleo_updated(&mut self, viewport_height: usize) -> Vec<DropdownItem> {
        let nucleo = &mut self.nucleo;

        nucleo.tick(10);
        let snapshot = nucleo.snapshot();

        // TODO: we should pass in the scroll_offset of the completion menu
        //   we'll leave it as 0 for now since it is already working well
        let scroll_offset = 0;

        snapshot
            .matched_items(
                scroll_offset as u32
                    ..viewport_height.min(snapshot.matched_item_count() as usize) as u32,
            )
            .map(|item| item.data.clone())
            .collect_vec()
    }

    fn new(task: PromptItemsBackgroundTask, notify: Callback<()>) -> Self {
        let nucleo = nucleo::Nucleo::new(
            nucleo::Config::DEFAULT,
            Arc::new(move || notify.call(())),
            None,
            1,
        );
        task.execute(nucleo.injector());
        PromptMatcher { nucleo }
    }
}

#[derive(Clone, PartialEq)]
pub struct PromptConfig {
    pub on_enter: PromptOnEnter,
    pub title: String,

    /// If defined, the `Dispatches` here is used for undoing the dispatches fired on change.
    pub on_cancelled: Option<Dispatches>,
    pub on_change: Option<PromptOnChangeDispatch>,
}

impl PromptConfig {
    pub fn new(title: String, on_enter: PromptOnEnter) -> Self {
        Self {
            title,
            on_enter,
            on_cancelled: Default::default(),
            on_change: Default::default(),
        }
    }
    pub fn items(&self) -> Vec<DropdownItem> {
        match &self.on_enter {
            PromptOnEnter::ParseCurrentLine {
                suggested_items, ..
            } => suggested_items.clone(),
            PromptOnEnter::ParseWholeBuffer { .. } => Vec::new(),
            PromptOnEnter::SelectsFirstMatchingItem { items } => match &items {
                PromptItems::None => Default::default(),
                PromptItems::Precomputed(dropdown_items) => dropdown_items.clone(),
                PromptItems::BackgroundTask { .. } => Default::default(),
            },
        }
    }

    pub fn set_on_change(self, on_change: Option<PromptOnChangeDispatch>) -> Self {
        Self { on_change, ..self }
    }

    pub fn set_on_cancelled(self, on_cancelled: Option<Dispatches>) -> PromptConfig {
        Self {
            on_cancelled,
            ..self
        }
    }
}

impl std::fmt::Debug for PromptConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptConfig")
            .field("title", &self.title)
            .field("on_cancelled", &self.on_cancelled)
            .finish()
    }
}

#[derive(Clone, PartialEq, Default)]
pub enum PromptItems {
    #[default]
    None,
    Precomputed(Vec<DropdownItem>),
    BackgroundTask {
        task: PromptItemsBackgroundTask,
        on_nucleo_tick_debounced: Callback<()>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum PromptItemsBackgroundTask {
    NonGitIgnoredFiles {
        working_directory: CanonicalizedPath,
    },
    HandledByMainEventLoop,
}
impl PromptItemsBackgroundTask {
    fn execute(self, injector: nucleo::Injector<DropdownItem>) {
        match self {
            PromptItemsBackgroundTask::NonGitIgnoredFiles { working_directory } => {
                std::thread::spawn(move || {
                    crate::list::WalkBuilderConfig::get_non_git_ignored_files(
                        working_directory.to_path_buf().clone(),
                        Arc::new(move |path_buf| {
                            let item = DropdownItem::from_path_buf(&working_directory, path_buf);
                            injector.push(item, |item, columns| {
                                let group = item.group().clone().unwrap_or_default();
                                let display = item.display().clone();
                                columns[0] = format!("{group} {display}").into();
                            });
                        }),
                    )
                });
            }
            PromptItemsBackgroundTask::HandledByMainEventLoop => {
                // Do nothing if the background task is handled by the app main event loop
                // For example: LSP Workspace Symbols
            }
        }
    }
}

#[derive(
    Hash, PartialEq, Eq, Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Default,
)]
pub enum PromptHistoryKey {
    MoveToIndex,
    Search,
    Rename,
    AddPath,
    MovePath,
    CopyFile,
    Symbol,
    OpenFile,
    CodeAction,
    #[default]
    Null,
    Theme,
    PipeToShell,
    FilterSelectionsMatchingSearch {
        maintain: bool,
    },
    KeyboardLayout,
    SurroundXmlTag,
    ResolveBufferSaveConflict,
    WorkspaceSymbol,
    ChangeWorkingDirectory,
}

impl Prompt {
    pub fn new(config: PromptConfig, context: &Context) -> (Self, Dispatches) {
        let (text, leaves_current_line_empty) = match &config.on_enter {
            PromptOnEnter::ParseCurrentLine {
                history_key,
                current_line,
                ..
            } => {
                let history = context.get_prompt_history(*history_key);
                let history = if history.is_empty() {
                    "".to_string()
                } else {
                    format!("{}\n", history.join("\n"))
                };
                let text = if let Some(current_line) = current_line {
                    if history.is_empty() {
                        current_line.to_string()
                    } else {
                        format!("{history}{current_line}")
                    }
                } else {
                    history
                };
                (text, current_line.is_none())
            }
            PromptOnEnter::ParseWholeBuffer { initial_lines, .. } => {
                (initial_lines.join("\n"), true)
            }
            PromptOnEnter::SelectsFirstMatchingItem { .. } => ("".to_string(), true),
        };
        let mut editor = SuggestiveEditor::from_buffer(
            Rc::new(RefCell::new(Buffer::new(None, &text))),
            SuggestiveEditorFilter::CurrentLine,
        );
        let dispatches = if leaves_current_line_empty {
            Dispatches::one(Dispatch::ToEditor(DispatchEditor::MoveToLastChar))
        } else {
            Dispatches::new(
                [
                    Dispatch::ToEditor(DispatchEditor::SetSelectionMode(
                        super::editor::IfCurrentNotFound::LookForward,
                        SelectionMode::Line,
                    )),
                    Dispatch::ToEditor(DispatchEditor::MoveSelection(
                        super::editor::Movement::Last,
                    )),
                    Dispatch::ToEditor(DispatchEditor::MoveToLineEnd),
                ]
                .to_vec(),
            )
        };
        // TODO: set cursor to last line
        editor.set_title(config.title.clone());
        editor.set_completion(Completion {
            items: config.items(),
            trigger_characters: vec![" ".to_string()],
        });
        let dispatches = dispatches.chain(editor.render_completion_dropdown(true));

        let matcher = if let PromptOnEnter::SelectsFirstMatchingItem {
            items:
                PromptItems::BackgroundTask {
                    task,
                    on_nucleo_tick_debounced,
                },
        } = &config.on_enter
        {
            let debounce = crate::thread::debounce(
                on_nucleo_tick_debounced.clone(),
                Duration::from_millis(1000 / 30), // 30 FPS
            );

            let matcher = PromptMatcher::new(task.clone(), debounce);
            Some(matcher)
        } else {
            None
        };

        (
            Prompt {
                editor,
                on_enter: config.on_enter,
                on_cancelled: config.on_cancelled,
                on_change: config.on_change,
                matcher,
            },
            dispatches,
        )
    }

    fn replace_current_query_with_focused_item(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> Result<Dispatches, anyhow::Error> {
        if self.editor.completion_dropdown_opened() {
            if let Some(item) = self.editor.completion_dropdown_current_item() {
                let dispatches = self.editor.update_current_line(context, &item.display())?;
                Ok(dispatches.chain(self.editor_mut().move_to_line_end()?))
            } else {
                Ok(Default::default())
            }
        } else {
            self.editor_mut().handle_key_event(context, event)
        }
    }

    pub fn render_completion_dropdown(&self) -> Dispatches {
        self.editor.render_completion_dropdown(true)
    }

    pub fn handle_nucleo_updated(&mut self, viewport_height: usize) -> Dispatches {
        let Some(matcher) = self.matcher.as_mut() else {
            return Default::default();
        };

        let items = matcher.handle_nucleo_updated(viewport_height);

        self.editor.update_items(items);

        self.render_completion_dropdown()
    }

    pub fn reparse_pattern(&mut self, filter: &str) {
        if let Some(matcher) = self.matcher.as_mut() {
            matcher.reparse(filter);
        }
    }

    pub fn clear_and_update_matcher_items(&mut self, items: Vec<DropdownItem>) {
        let Some(matcher) = self.matcher.as_mut() else {
            return Default::default();
        };

        matcher.nucleo.restart(true);

        let injector = matcher.nucleo.injector();
        for item in items {
            injector.push(item, |item, columns| {
                let group = item.group().clone().unwrap_or_default();
                let display = item.display().clone();
                columns[0] = format!("{group} {display}").into();
            });
        }
    }

    pub fn get_on_change_dispatches(&self) -> Dispatches {
        self.on_change
            .as_ref()
            .map(|on_change| {
                on_change.to_dispatches(&PromptContext {
                    current_line: self.editor().current_line().unwrap_or_default(),
                })
            })
            .unwrap_or_default()
    }
}

impl Component for Prompt {
    fn editor(&self) -> &Editor {
        self.editor.editor()
    }

    fn editor_mut(&mut self) -> &mut Editor {
        self.editor.editor_mut()
    }

    fn handle_dispatch_editor(
        &mut self,
        context: &mut Context,
        dispatch: super::editor::DispatchEditor,
    ) -> anyhow::Result<Dispatches> {
        self.editor.handle_dispatch_editor(context, dispatch)
    }

    fn handle_key_event(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        match event {
            key!("esc") if self.editor().mode == Mode::Normal => {
                Ok(Dispatches::one(Dispatch::CloseCurrentWindow)
                    .chain(self.on_cancelled.clone().unwrap_or_default()))
            }
            key!("tab") => self.replace_current_query_with_focused_item(context, event),
            _ if event.display() == alted("x") => {
                self.replace_current_query_with_focused_item(context, event)
            }
            key!("enter") => {
                let dispatches = match &self.on_enter {
                    PromptOnEnter::SelectsFirstMatchingItem { .. } => self
                        .editor
                        .completion_dropdown_current_item()
                        .map(|item| item.dispatches)
                        .unwrap_or_default(),
                    PromptOnEnter::ParseCurrentLine {
                        parser,
                        history_key: prompt_history_key,
                        ..
                    } => {
                        let entry = self.editor().current_line()?;
                        parser.parse(&entry)?.append(Dispatch::PushPromptHistory {
                            key: *prompt_history_key,
                            line: entry.clone(),
                        })
                    }
                    PromptOnEnter::ParseWholeBuffer { parser, .. } => {
                        let entry = self.editor().content();
                        parser.parse(&entry)?
                    }
                };

                Ok(Dispatches::one(Dispatch::CloseCurrentWindow).chain(dispatches))
            }
            _ => {
                let dispatches = self.editor.handle_key_event(context, event)?;
                Ok(dispatches.chain(
                    self.editor
                        .completion_dropdown_current_item()
                        .map(|item| item.on_focused())
                        .unwrap_or_default(),
                ))
            }
        }
    }

    fn post_handle_event(&self, dispatches: Dispatches) -> anyhow::Result<Dispatches> {
        Ok(dispatches
            .append(Dispatch::GetAndHandlePromptOnChangeDispatches)
            .append(Dispatch::ToSuggestiveEditor(
                DispatchSuggestiveEditor::UpdateFilter,
            )))
    }
}
impl Prompt {
    pub fn handle_dispatch_suggestive_editor(
        &mut self,
        dispatch: DispatchSuggestiveEditor,
    ) -> anyhow::Result<Dispatches> {
        self.editor.handle_dispatch(dispatch)
    }
}
#[cfg(test)]
mod test_prompt {
    use crate::{
        app::{LocalSearchConfigUpdate, Scope},
        buffer::BufferOwner,
        components::{
            editor::{Direction, IfCurrentNotFound},
            suggestive_editor::Info,
        },
        list::grep::RegexConfig,
        lsp::completion::CompletionItem,
        test_app::*,
    };
    use my_proc_macros::keys;

    use crate::{app::Dispatch, position::Position};

    use super::*;

    #[test]
    fn leaves_current_line_empty_if_current_line_is_not_defined() {
        fn test(
            leaves_current_line_empty: bool,
            expected_text: &'static str,
            expected_cursor_position: Position,
        ) {
            execute_test(|s| {
                Box::new([
                    App(OpenFile {
                        path: s.main_rs(),
                        owner: BufferOwner::User,
                        focus: true,
                    }),
                    App(OpenPrompt {
                        config: PromptConfig::new(
                            "".to_string(),
                            PromptOnEnter::ParseCurrentLine {
                                parser: DispatchParser::AddPath,
                                history_key: PromptHistoryKey::AddPath,
                                current_line: if leaves_current_line_empty {
                                    None
                                } else {
                                    Some("hello\nworld".to_string())
                                },
                                suggested_items: Vec::new(),
                            },
                        ),
                    }),
                    Expect(CurrentComponentContent(expected_text)),
                    Expect(EditorCursorPosition(expected_cursor_position)),
                ])
            })
            .unwrap();
        }
        test(true, "", Position::new(0, 0));
        test(false, "hello\nworld", Position::new(1, 5));
    }

    #[test]
    fn prompt_history() -> anyhow::Result<()> {
        execute_test(|s| {
            let open_prompt = OpenPrompt {
                config: PromptConfig::new(
                    "".to_string(),
                    PromptOnEnter::ParseCurrentLine {
                        parser: DispatchParser::Null,
                        history_key: PromptHistoryKey::Null,
                        current_line: None,
                        suggested_items: Vec::new(),
                    },
                ),
            };
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(open_prompt.clone()),
                App(HandleKeyEvents(keys!("h e l l o enter").to_vec())),
                App(open_prompt.clone()),
                Expect(CurrentComponentContent("hello\n")),
                App(HandleKeyEvents(keys!("h e l l o enter").to_vec())),
                App(open_prompt.clone()),
                // Duplicates should not be repeated
                Expect(CurrentComponentContent("hello\n")),
                App(open_prompt.clone()),
                App(HandleKeyEvents(keys!("y o enter").to_vec())),
                App(open_prompt.clone()),
                // Latest entry should appear last
                Expect(CurrentComponentContent("hello\nyo\n")),
                // Enter 'hello' again
                App(open_prompt.clone()),
                App(HandleKeyEvents(keys!("h e l l o enter").to_vec())),
                // 'hello' should appear last
                App(open_prompt.clone()),
                Expect(CurrentComponentContent("yo\nhello\n")),
            ])
        })
    }

    #[test]
    fn should_not_contain_newline_if_empty() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                App(Dispatch::OpenPrompt {
                    config: super::PromptConfig::new(
                        "".to_string(),
                        PromptOnEnter::ParseCurrentLine {
                            parser: DispatchParser::Null,
                            history_key: PromptHistoryKey::Null,
                            current_line: None,
                            suggested_items: Vec::new(),
                        },
                    ),
                }),
                Expect(CurrentComponentContent("")),
            ])
        })
    }

    #[test]
    fn enter_selects_first_matching_item() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(OpenPrompt {
                    config: super::PromptConfig::new(
                        "".to_string(),
                        PromptOnEnter::SelectsFirstMatchingItem {
                            items: PromptItems::Precomputed(
                                ["foo".to_string(), "bar".to_string()]
                                    .into_iter()
                                    .map(|str| {
                                        let item: DropdownItem = str.clone().into();
                                        item.set_dispatches(Dispatches::one(ToEditor(SetContent(
                                            str,
                                        ))))
                                    })
                                    .collect(),
                            ),
                        },
                    ),
                }),
                Expect(CompletionDropdownIsOpen(true)),
                App(HandleKeyEvents(event::parse_key_events("f").unwrap())),
                App(HandleKeyEvent(key!("enter"))),
                Expect(CurrentComponentContent("foo")),
            ])
        })
    }

    #[test]
    fn tab_replace_current_content_with_first_highlighted_suggestion() -> anyhow::Result<()> {
        execute_test(|_| {
            Box::new([
                App(Dispatch::OpenPrompt {
                    config: super::PromptConfig::new(
                        "".to_string(),
                        PromptOnEnter::ParseCurrentLine {
                            parser: DispatchParser::SetContent,
                            history_key: PromptHistoryKey::Null,
                            current_line: None,
                            suggested_items: ["foo_bar".to_string()]
                                .into_iter()
                                .map(|item| item.into())
                                .collect(),
                        },
                    ),
                }),
                App(HandleKeyEvents(keys!("f o o _ b tab").to_vec())),
                Expect(CurrentComponentContent("foo_bar")),
                Expect(EditorCursorPosition(Position { line: 0, column: 7 })),
            ])
        })
    }

    #[test]
    fn test_on_cancelled() -> anyhow::Result<()> {
        execute_test(|_| {
            Box::new([
                App(Dispatch::OpenPrompt {
                    config: super::PromptConfig::new(
                        "".to_string(),
                        PromptOnEnter::SelectsFirstMatchingItem {
                            items: PromptItems::Precomputed(
                                [
                                    "foo_bar".to_string(),
                                    "zazam".to_string(),
                                    "boque".to_string(),
                                ]
                                .into_iter()
                                .map(|item| item.into())
                                .map(|item: DropdownItem| {
                                    let content = item.display();
                                    item.set_on_focused(Dispatches::one(Dispatch::ShowEditorInfo(
                                        Info::new("".to_string(), content),
                                    )))
                                })
                                .collect(),
                            ),
                        },
                    )
                    .set_on_cancelled(Some(Dispatches::one(
                        Dispatch::ShowEditorInfo(Info::new(
                            "".to_string(),
                            "back to square one".to_string(),
                        )),
                    ))),
                }),
                App(HandleKeyEvents(keys!("f o o _").to_vec())),
                Expect(EditorInfoContents(&["foo_bar"])),
                App(HandleKeyEvents(keys!("alt+q z a m").to_vec())),
                Expect(EditorInfoContents(&["zazam"])),
                App(HandleKeyEvents(keys!("alt+q q").to_vec())),
                Expect(EditorInfoContents(&["boque"])),
                App(HandleKeyEvents(keys!("esc esc").to_vec())),
                Expect(EditorInfoContents(&["back to square one"])),
            ])
        })
    }

    #[test]
    fn filter_without_matching_items_should_clear_suggestions() -> anyhow::Result<()> {
        execute_test(|_| {
            Box::new([
                App(Dispatch::OpenPrompt {
                    config: super::PromptConfig::new(
                        "".to_string(),
                        PromptOnEnter::ParseCurrentLine {
                            parser: DispatchParser::SetContent,
                            history_key: PromptHistoryKey::Null,
                            current_line: None,
                            suggested_items: [CompletionItem::from_label(
                                "spongebob squarepants".to_string(),
                            )]
                            .into_iter()
                            .map(|item| item.into())
                            .collect(),
                        },
                    ),
                }),
                App(TerminalDimensionChanged(crate::app::Dimension {
                    height: 10,
                    width: 50,
                })),
                Expect(AppGridContains("squarepants")),
                App(HandleKeyEvents(keys!("f o o").to_vec())),
                Expect(Not(Box::new(AppGridContains("squarepants")))),
            ])
        })
    }

    #[test]
    fn suggestion_should_update_with_alt_q_and_alt_t() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                App(Dispatch::OpenPrompt {
                    config: super::PromptConfig::new(
                        "".to_string(),
                        PromptOnEnter::ParseCurrentLine {
                            parser: DispatchParser::SetContent,
                            history_key: PromptHistoryKey::Null,
                            current_line: None,
                            suggested_items: ["Patrick", "Spongebob", "Squidward"]
                                .into_iter()
                                .map(|item| item.to_string().into())
                                .collect(),
                        },
                    ),
                }),
                // Expect the completion dropdown to be open,
                Expect(CompletionDropdownContent("Patrick\nSpongebob\nSquidward")),
                // Type in 'pa'
                App(HandleKeyEvents(keys!("p a").to_vec())),
                // Expect only 'Patrick' remains in the completion dropdown
                Expect(CompletionDropdownContent("Patrick")),
                // Clear 'pa' using alt+a
                App(HandleKeyEvent(key!("alt+q"))),
                // Expect all items are shown again
                Expect(CompletionDropdownContent("Patrick\nSpongebob\nSquidward")),
                //
                //
                // Perform the same test for alt+g
                App(HandleKeyEvents(keys!("p a").to_vec())),
                Expect(CompletionDropdownContent("Patrick")),
                App(HandleKeyEvents(keys!("alt+s alt+t").to_vec())),
                Expect(CompletionDropdownContent("Patrick\nSpongebob\nSquidward")),
            ])
        })
    }

    #[test]
    fn replace_current_query_with_focused_item_should_replace_only_current_line(
    ) -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo bar spam".to_string())),
                App(OpenSearchPrompt {
                    scope: Scope::Local,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                }),
                App(HandleKeyEvents(keys!("f o enter").to_vec())), // Populate search history with "fo"
                App(OpenSearchPrompt {
                    scope: Scope::Local,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                }),
                Expect(CurrentComponentContent("fo\n")),
                App(HandleKeyEvents(keys!("f o alt+x").to_vec())),
                Expect(CurrentComponentContent("fo\nfoo")),
            ])
        })
    }

    #[test]
    fn using_history_entries() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foox bar spam fooy".to_string())),
                App(UpdateLocalSearchConfig {
                    update: LocalSearchConfigUpdate::Mode(
                        crate::context::LocalSearchConfigMode::Regex(RegexConfig {
                            escaped: false,
                            case_sensitive: false,
                            match_whole_word: false,
                        }),
                    ),
                    scope: Scope::Local,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                    run_search_after_config_updated: false,
                    component_id: None,
                }),
                App(OpenSearchPrompt {
                    scope: Scope::Local,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                }),
                App(HandleKeyEvents(keys!("f o o . enter").to_vec())), // Populate search history with "foo."
                App(OpenSearchPrompt {
                    scope: Scope::Local,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                }),
                Expect(CurrentComponentContent("foo.\n")),
                // Navigate upwards and use the history
                Editor(EnterNormalMode),
                Editor(MoveSelection(Left)),
                App(HandleKeyEvent(key!("enter"))),
                Expect(CurrentSearch(Scope::Local, "foo.")),
            ])
        })
    }
}
