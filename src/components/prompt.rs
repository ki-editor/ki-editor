use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use itertools::Itertools;
use my_proc_macros::key;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    app::{Dispatch, DispatchPrompt, Dispatches},
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
    editor_keymap::Meaning,
    suggestive_editor::{DispatchSuggestiveEditor, SuggestiveEditor, SuggestiveEditorFilter},
};

pub(crate) struct Prompt {
    editor: SuggestiveEditor,
    /// This will only be run on user input if the user input matches no dropdown item.
    on_enter: DispatchPrompt,
    enter_selects_first_matching_item: bool,
    prompt_history_key: PromptHistoryKey,
    on_change: Option<PromptOnChangeDispatch>,
    on_cancelled: Option<Dispatches>,
    matcher: Option<PromptMatcher>,
}

#[derive(Debug, Clone)]
pub(crate) enum PromptOnChangeDispatch {
    RequestWorkspaceSymbol(CanonicalizedPath),
    SetIncrementalSearchConfig { component_id: ComponentId },
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
        }
    }
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

    fn handle_nucleo_updated(&mut self, viewport_height: u32) -> Vec<DropdownItem> {
        let nucleo = &mut self.nucleo;

        nucleo.tick(10);
        let snapshot = nucleo.snapshot();

        // TODO: we should pass in the scroll_offset of the completion menu
        //   we'll leave it as 0 for now since it is already working well
        let scroll_offset = 0;

        snapshot
            .matched_items(scroll_offset..viewport_height.min(snapshot.matched_item_count()))
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

#[derive(Clone, Default)]
pub(crate) struct PromptConfig {
    pub(crate) on_enter: DispatchPrompt,
    pub(crate) items: PromptItems,
    pub(crate) title: String,
    pub(crate) enter_selects_first_matching_item: bool,
    pub(crate) leaves_current_line_empty: bool,

    /// If defined, the `Dispatches` here is used for undoing the dispatches fired on change.
    pub(crate) on_cancelled: Option<Dispatches>,
    pub(crate) prompt_history_key: PromptHistoryKey,
    pub(crate) on_change: Option<PromptOnChangeDispatch>,
}

impl PromptConfig {
    pub(crate) fn items(&self) -> Vec<DropdownItem> {
        match &self.items {
            PromptItems::None => Default::default(),
            PromptItems::Precomputed(dropdown_items) => dropdown_items.clone(),
            PromptItems::BackgroundTask { .. } => Default::default(),
        }
    }
}

impl PartialEq for PromptConfig {
    fn eq(&self, other: &Self) -> bool {
        self.on_enter == other.on_enter && self.title == other.title
    }
}

impl std::fmt::Debug for PromptConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptConfig")
            .field("on_enter", &self.on_enter)
            .field("title", &self.title)
            .field(
                "enter_selects_first_matching_item",
                &self.enter_selects_first_matching_item,
            )
            .field("leaves_current_line_empty", &self.leaves_current_line_empty)
            .field("on_cancelled", &self.on_cancelled)
            .field("prompt_history_key", &self.prompt_history_key)
            .finish()
    }
}

#[derive(Clone)]
pub(crate) enum PromptItems {
    None,
    Precomputed(Vec<DropdownItem>),
    BackgroundTask {
        task: PromptItemsBackgroundTask,
        on_nucleo_tick_debounced: Callback<()>,
    },
}

impl Default for PromptItems {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PromptItemsBackgroundTask {
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

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub(crate) enum PromptHistoryKey {
    MoveToIndex,
    Search,
    Rename,
    AddPath,
    MovePath,
    CopyFile,
    Symbol,
    OpenFile,
    CodeAction,
    Null,
    Theme,
    PipeToShell,
    FilterSelectionsMatchingSearch { maintain: bool },
    KeyboardLayout,
    SurroundXmlTag,
    ResolveBufferSaveConflict,
    WorkspaceSymbol,
}

impl Default for PromptHistoryKey {
    fn default() -> Self {
        Self::Null
    }
}

impl Prompt {
    pub(crate) fn new(config: PromptConfig, history: Vec<String>) -> (Self, Dispatches) {
        let text = {
            if history.is_empty() {
                "".to_string()
            } else {
                format!(
                    "{}{}",
                    history.join("\n"),
                    if config.leaves_current_line_empty {
                        "\n"
                    } else {
                        ""
                    }
                )
            }
        };
        let mut editor = SuggestiveEditor::from_buffer(
            Rc::new(RefCell::new(Buffer::new(None, &text))),
            SuggestiveEditorFilter::CurrentLine,
        );
        let dispatches = if config.leaves_current_line_empty {
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

        let matcher = if let PromptItems::BackgroundTask {
            task,
            on_nucleo_tick_debounced,
        } = config.items
        {
            let debounce = crate::thread::debounce(
                on_nucleo_tick_debounced,
                Duration::from_millis(1000 / 30), // 30 FPS
            );

            let matcher = PromptMatcher::new(task, debounce);
            Some(matcher)
        } else {
            None
        };

        (
            Prompt {
                editor,
                on_enter: config.on_enter,
                enter_selects_first_matching_item: config.enter_selects_first_matching_item,
                prompt_history_key: config.prompt_history_key,
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

    pub(crate) fn render_completion_dropdown(&self) -> Dispatches {
        self.editor.render_completion_dropdown(true)
    }

    pub(crate) fn handle_nucleo_updated(&mut self, viewport_height: u32) -> Dispatches {
        let Some(matcher) = self.matcher.as_mut() else {
            return Default::default();
        };

        let items = matcher.handle_nucleo_updated(viewport_height);

        self.editor.update_items(items);

        self.render_completion_dropdown()
    }

    pub(crate) fn reparse_pattern(&mut self, filter: &str) {
        if let Some(matcher) = self.matcher.as_mut() {
            matcher.reparse(filter);
        }
    }

    pub(crate) fn clear_and_update_matcher_items(&mut self, items: Vec<DropdownItem>) {
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

    pub(crate) fn get_on_change_dispatches(&self) -> Dispatches {
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
            _ if event.display() == context.keyboard_layout_kind().get_key(&Meaning::MrkFN) => {
                self.replace_current_query_with_focused_item(context, event)
            }
            key!("enter") => {
                let (line, dispatches) = if self.enter_selects_first_matching_item
                    && self.editor.completion_dropdown_current_item().is_some()
                {
                    self.editor
                        .completion_dropdown_current_item()
                        .map(|item| (item.display(), item.dispatches))
                        .unwrap_or_default()
                } else {
                    let current_line = self.editor().current_line()?;
                    let dispatches = self.on_enter.to_dispatches(&current_line)?;
                    (current_line, dispatches)
                };

                Ok(Dispatches::one(Dispatch::CloseCurrentWindow)
                    .chain(dispatches)
                    .append(Dispatch::PushPromptHistory {
                        key: self.prompt_history_key,
                        line,
                    }))
            }
            _ if self.prompt_history_key == PromptHistoryKey::OpenFile
                && event.display() == context.keyboard_layout_kind().get_key(&Meaning::OpenM) =>
            {
                Ok(
                    Dispatches::one(Dispatch::CloseCurrentWindow).chain(Dispatches::new(
                        self.editor
                            .all_filtered_items()
                            .into_iter()
                            .flat_map(|item| item.dispatches.into_vec())
                            .collect(),
                    )),
                )
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
        Ok(dispatches.append(Dispatch::GetAndHandlePromptOnChangeDispatches))
    }
}
impl Prompt {
    pub(crate) fn handle_dispatch_suggestive_editor(
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
    fn leaves_current_line_empty() {
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
                        current_line: Some("hello\nworld".to_string()),
                        config: PromptConfig {
                            enter_selects_first_matching_item: true,
                            leaves_current_line_empty,
                            ..Default::default()
                        },
                    }),
                    Expect(CurrentComponentContent(expected_text)),
                    Expect(EditorCursorPosition(expected_cursor_position)),
                ])
            })
            .unwrap();
        }
        test(true, "hello\nworld\n", Position::new(2, 0));
        test(false, "hello\nworld", Position::new(1, 5));
    }

    #[test]
    fn prompt_history() {
        execute_test(|s| {
            let open_prompt = OpenPrompt {
                current_line: None,
                config: PromptConfig {
                    enter_selects_first_matching_item: true,
                    leaves_current_line_empty: true,
                    ..Default::default()
                },
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
        .unwrap();
    }

    #[test]
    fn current_line() {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App((OpenPrompt {
                    current_line: Some("spongebob squarepants".to_string()),
                    config: PromptConfig {
                        enter_selects_first_matching_item: true,
                        leaves_current_line_empty: true,
                        ..Default::default()
                    },
                })
                .clone()),
                Expect(CurrentComponentContent("spongebob squarepants\n")),
            ])
        })
        .unwrap();
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
                    current_line: None,
                    config: super::PromptConfig {
                        on_enter: DispatchPrompt::SetContent,
                        enter_selects_first_matching_item: true,
                        leaves_current_line_empty: true,
                        ..Default::default()
                    },
                }),
                Expect(CurrentComponentContent("")),
            ])
        })
    }

    #[test]
    fn enter_selects_first_matching_item() {
        fn run_test(
            enter_selects_first_matching_item: bool,
            input_text: &str,
            expected_invoked_text: &'static str,
        ) {
            execute_test(|s| {
                Box::new([
                    App(OpenFile {
                        path: s.main_rs(),
                        owner: BufferOwner::User,
                        focus: true,
                    }),
                    App(OpenPrompt {
                        current_line: None,
                        config: super::PromptConfig {
                            on_enter: DispatchPrompt::SetContent,
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

                            enter_selects_first_matching_item,
                            leaves_current_line_empty: true,
                            ..Default::default()
                        },
                    }),
                    Expect(CompletionDropdownIsOpen(true)),
                    App(HandleKeyEvents(
                        event::parse_key_events(input_text).unwrap(),
                    )),
                    App(HandleKeyEvent(key!("enter"))),
                    Expect(CurrentComponentContent(expected_invoked_text)),
                ])
            })
            .unwrap()
        }

        run_test(true, "f", "foo");
        run_test(false, "f", "f");
    }

    #[test]
    fn ctrl_space_replace_current_content_with_first_highlighted_suggestion() -> anyhow::Result<()>
    {
        execute_test(|_| {
            Box::new([
                App(Dispatch::OpenPrompt {
                    current_line: None,
                    config: super::PromptConfig {
                        on_enter: DispatchPrompt::SetContent,
                        items: PromptItems::Precomputed(
                            ["foo_bar".to_string()]
                                .into_iter()
                                .map(|item| item.into())
                                .collect(),
                        ),

                        enter_selects_first_matching_item: true,
                        leaves_current_line_empty: true,
                        ..Default::default()
                    },
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
                    current_line: None,
                    config: super::PromptConfig {
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

                        enter_selects_first_matching_item: true,
                        leaves_current_line_empty: true,
                        on_cancelled: Some(Dispatches::one(Dispatch::ShowEditorInfo(Info::new(
                            "".to_string(),
                            "back to square one".to_string(),
                        )))),
                        ..Default::default()
                    },
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
                    current_line: None,
                    config: super::PromptConfig {
                        on_enter: DispatchPrompt::SetContent,
                        items: PromptItems::Precomputed(
                            [CompletionItem::from_label(
                                "spongebob squarepants".to_string(),
                            )]
                            .into_iter()
                            .map(|item| item.into())
                            .collect(),
                        ),

                        enter_selects_first_matching_item: true,
                        leaves_current_line_empty: true,
                        ..Default::default()
                    },
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
                    current_line: None,
                    config: super::PromptConfig {
                        on_enter: DispatchPrompt::SetContent,
                        items: PromptItems::Precomputed(
                            ["Patrick", "Spongebob", "Squidward"]
                                .into_iter()
                                .map(|item| item.to_string().into())
                                .collect(),
                        ),

                        enter_selects_first_matching_item: true,
                        leaves_current_line_empty: true,
                        ..Default::default()
                    },
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
                App(HandleKeyEvents(keys!("f o alt+l").to_vec())),
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
                Editor(MoveSelection(Up)),
                App(HandleKeyEvent(key!("enter"))),
                Expect(CurrentSearch(Scope::Local, "foo.")),
            ])
        })
    }
}
