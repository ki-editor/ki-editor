use crate::{
    buffer::{Buffer, BufferOwner},
    char_index_range::CharIndexRange,
    clipboard::CopiedTexts,
    components::{
        component::{Component, ComponentId, GetGridResult},
        dropdown::{DropdownItem, DropdownRender},
        editor::{
            Direction, DispatchEditor, Editor, IfCurrentNotFound, Movement, PriorChange, Reveal,
        },
        editor_keymap::KeyboardLayoutKind,
        editor_keymap_printer::KeymapDisplayOption,
        file_explorer::FileExplorer,
        keymap_legend::{Keybinding, Keymap, KeymapLegendConfig, ReleaseKey},
        prompt::{
            Prompt, PromptConfig, PromptHistoryKey, PromptItems, PromptItemsBackgroundTask,
            PromptOnChangeDispatch, PromptOnEnter,
        },
        suggestive_editor::{
            DispatchSuggestiveEditor, Info, SuggestiveEditor, SuggestiveEditorFilter,
        },
    },
    context::{
        Context, GlobalMode, GlobalSearchConfig, LocalSearchConfigMode, QuickfixListSource, Search,
    },
    edit::Edit,
    file_watcher::{FileWatcherEvent, FileWatcherInput},
    frontend::Frontend,
    git::{self},
    grid::{Grid, LineUpdate},
    integration_event::{IntegrationEvent, IntegrationEventEmitter},
    layout::Layout,
    list::{self, Match, WalkBuilderConfig},
    lsp::{
        completion::CompletionItem,
        goto_definition_response::GotoDefinitionResponse,
        manager::LspManager,
        process::{FromEditor, LspNotification, ResponseContext},
        symbols::Symbols,
        workspace_edit::WorkspaceEdit,
    },
    persistence::Persistence,
    position::Position,
    quickfix_list::{Location, QuickfixListItem, QuickfixListType},
    render_flex_layout::{self, FlexLayoutComponent},
    screen::{Screen, Window},
    scripting::{custom_keymap, ScriptDispatch, ScriptInput},
    search::parse_search_config,
    selection::{CharIndex, SelectionMode},
    syntax_highlight::{HighlightedSpans, SyntaxHighlightRequest, SyntaxHighlightRequestBatchId},
    thread::{debounce, Callback, SendResult},
    ui_tree::{ComponentKind, KindedComponent},
};
use event::event::Event;
use indexmap::IndexMap;
use itertools::{Either, Itertools};
use my_proc_macros::NamedVariant;
use nonempty::NonEmpty;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use shared::language::LanguageId;
use shared::{canonicalized_path::CanonicalizedPath, language::Language};
use std::{
    any::TypeId,
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};
use std::{sync::Arc, time::Duration};
use strum::IntoEnumIterator;
use DispatchEditor::*;

#[cfg(test)]
use crate::{layout::BufferContentsMap, quickfix_list::QuickfixList, test_app::RunTestOptions};

// TODO: rename current Context struct to RawContext struct
// The new Context struct should always be derived, it should contains Hashmap of rectangles, keyed by Component ID
// The scroll offset of each componentn should only be recalculated when:
// 1. The number of components is changed (this means we need to store the components)
// 2. The terminal dimension is changed
pub struct App<T: Frontend> {
    context: Context,

    sender: Sender<AppMessage>,

    /// Used for receiving message from various sources:
    /// - Events from crossterm
    /// - Notifications from language server
    receiver: Receiver<AppMessage>,

    /// Sender for integration events (used by external integrations like VSCode)
    integration_event_sender: Option<Sender<crate::integration_event::IntegrationEvent>>,

    /// Each working directory should have its own LSP manager
    /// so that the LSPs functionalities still work when user change the working directory
    lsp_manager: HashMap<PathBuf, LspManager>,
    enable_lsp: bool,

    global_title: Option<String>,

    layout: Layout,

    frontend: Rc<Mutex<T>>,

    syntax_highlight_request_sender: Option<Sender<SyntaxHighlightRequest>>,
    status_lines: Vec<StatusLine>,
    last_action_description: Option<String>,
    last_action_short_description: Option<String>,

    /// This is necessary when Ki is running as an embedded application
    last_prompt_config: Option<PromptConfig>,

    /// This is used for suspending events until the buffer content
    /// is synced between Ki and the host application.
    queued_events: Vec<Event>,
    file_watcher_input_sender: Option<Sender<FileWatcherInput>>,
    /// Used for debouncing LSP Completion request, so that we don't overwhelm
    /// the server with too many requests, and also Ki with too many incoming Completion responses
    debounce_lsp_request_completion: Callback<()>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct StatusLine {
    components: Vec<StatusLineComponent>,
}
impl StatusLine {
    #[cfg(test)]
    pub fn new(components: Vec<StatusLineComponent>) -> Self {
        Self { components }
    }
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub enum StatusLineComponent {
    KiCharacter,
    CurrentWorkingDirectory,
    GitBranch,
    Mode,
    SelectionMode,
    LastDispatch,
    LineColumn,
    LastSearchString,
    Help,
    KeyboardLayout,
    Reveal,
    /// A spacer pushes its preceding group of components to the left,
    /// and the following to the right.
    ///
    /// If a status line contains more than one spacers,
    /// each spacer will be given the similar width.
    Spacer,
    CurrentFileParentFolder,
    LspProgress,
    /// The detected language for the current file
    Language,
}

impl<T: Frontend> App<T> {
    #[cfg(test)]
    pub fn new(
        frontend: Rc<Mutex<T>>,
        working_directory: CanonicalizedPath,
        status_lines: Vec<StatusLine>,
        options: RunTestOptions,
    ) -> anyhow::Result<App<T>> {
        use crate::syntax_highlight;

        let (sender, receiver) = std::sync::mpsc::channel();
        let syntax_highlight_request_sender = if options.enable_syntax_highlighting {
            Some(syntax_highlight::start_thread(sender.clone()))
        } else {
            None
        };
        Self::from_channel(
            frontend,
            working_directory,
            sender,
            receiver,
            syntax_highlight_request_sender,
            status_lines,
            None, // No integration event sender
            options.enable_lsp,
            options.enable_file_watcher,
            false,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_channel(
        frontend: Rc<Mutex<T>>,
        working_directory: CanonicalizedPath,
        sender: Sender<AppMessage>,
        receiver: Receiver<AppMessage>,
        syntax_highlight_request_sender: Option<Sender<SyntaxHighlightRequest>>,
        status_lines: Vec<StatusLine>,
        integration_event_sender: Option<Sender<crate::integration_event::IntegrationEvent>>,
        enable_lsp: bool,
        enable_file_watcher: bool,
        is_running_as_embedded: bool,
        persistence: Option<Persistence>,
    ) -> anyhow::Result<App<T>> {
        let dimension = frontend.lock().unwrap().get_terminal_dimension()?;
        let file_watcher_input_sender = if enable_file_watcher {
            Some(crate::file_watcher::watch_file_changes(
                &working_directory.clone(),
                sender.clone(),
            )?)
        } else {
            None
        };
        let mut app = App {
            context: Context::new(
                working_directory.clone(),
                is_running_as_embedded,
                persistence,
            ),
            receiver,
            lsp_manager: [(
                working_directory.clone().into_path_buf(),
                LspManager::new(sender.clone(), working_directory.clone()),
            )]
            .into_iter()
            .collect(),
            enable_lsp,
            debounce_lsp_request_completion: {
                let sender = sender.clone();
                debounce(
                    Callback::new(Arc::new(move |_| {
                        if let Err(err) = sender.send(AppMessage::ExternalDispatch(Box::new(
                            Dispatch::RequestCompletionDebounced,
                        ))) {
                            log::error!(
                                "Failed to send RequestCompletionDebounced to App due to {err:?}"
                            )
                        }
                    })),
                    Duration::from_millis(300),
                )
            },
            sender,
            layout: Layout::new(
                dimension.decrement_height(status_lines.len()),
                &working_directory,
            )?,
            frontend,
            syntax_highlight_request_sender,
            global_title: None,
            status_lines,
            last_action_description: None,
            last_action_short_description: None,
            integration_event_sender,
            last_prompt_config: None,
            queued_events: Vec::new(),
            file_watcher_input_sender,
        };

        app.restore_session();

        Ok(app)
    }

    fn global_title_bar_height(&self) -> usize {
        self.status_lines.len()
    }

    fn update_highlighted_spans(
        &self,
        component_id: ComponentId,
        batch_id: SyntaxHighlightRequestBatchId,
        highlighted_spans: HighlightedSpans,
    ) -> Result<(), anyhow::Error> {
        self.layout
            .update_highlighted_spans(component_id, batch_id, highlighted_spans)
    }

    fn set_terminal_options(&mut self) -> anyhow::Result<()> {
        let mut frontend = self.frontend.lock().unwrap();
        frontend.enter_alternate_screen()?;
        frontend.enable_raw_mode()?;
        frontend.enable_mouse_capture()?;
        Ok(())
    }

    /// This is the main event loop.
    pub fn run(mut self, entry_path: Option<CanonicalizedPath>) -> Result<(), anyhow::Error> {
        self.set_terminal_options()?;

        if let Some(entry_path) = entry_path {
            if entry_path.as_ref().is_dir() {
                self.layout.open_file_explorer();
            } else {
                self.open_file(&entry_path, BufferOwner::User, true, true)?;
            }
        }

        self.render()?;

        while let Ok(message) = self.receiver.recv() {
            let should_quit = self.process_message(message).unwrap_or_else(|e| {
                self.show_global_info(Info::new("ERROR".to_string(), e.to_string()));
                false
            });

            if should_quit || self.should_quit() {
                break;
            }

            self.render()?;
        }

        self.quit()
    }

    pub fn process_message(&mut self, message: AppMessage) -> anyhow::Result<bool> {
        match message {
            AppMessage::Event(event) => self.handle_event(event),
            AppMessage::LspNotification(notification) => {
                self.handle_lsp_notification(*notification).map(|_| false)
            }
            AppMessage::QuitAll => {
                self.quit()?;
                Ok(true)
            }
            AppMessage::SyntaxHighlightResponse {
                component_id,
                batch_id,
                highlighted_spans,
            } => self
                .update_highlighted_spans(component_id, batch_id, highlighted_spans)
                .map(|_| false),
            AppMessage::ExternalDispatch(dispatch) => {
                // Process the dispatch directly
                self.handle_dispatch(*dispatch)?;
                Ok(false)
            }
            AppMessage::NucleoTickDebounced => {
                self.handle_nucleo_debounced()?;
                Ok(false)
            }
            AppMessage::FileWatcherEvent(event) => {
                self.handle_file_watcher_event(event)?;
                Ok(false)
            }
            AppMessage::NotifyError(error) => {
                self.show_global_info(Info::new("App Error".to_string(), format!("{error:#?}")));
                Ok(false)
            }
        }
    }

    fn prepare_to_suspend_or_quit(&mut self) -> anyhow::Result<()> {
        let mut frontend = self.frontend.lock().unwrap();
        frontend.leave_alternate_screen()?;
        frontend.disable_raw_mode()?;
        frontend.disable_mouse_capture()?;
        self.context.persist_data();
        Ok(())
    }

    pub fn quit(&mut self) -> anyhow::Result<()> {
        self.prepare_to_suspend_or_quit()?;

        self.lsp_manager().shutdown();

        Ok(())
    }

    #[cfg(windows)]
    fn suspend(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Send SIGSTOP to the current process group to stop the editor.
    /// After receiving SIGCONT, continue.
    #[cfg(unix)]
    fn suspend(&mut self) -> anyhow::Result<()> {
        self.prepare_to_suspend_or_quit()?;

        // Copy Helix's behaviour here.
        let code = unsafe {
            // Rationale: https://github.com/helix-editor/helix/blob/036729211a94d058b835f5ee212ab15de83bc037/helix-term/src/application.rs#L481
            libc::kill(0, libc::SIGSTOP)
        };

        if code != 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        // Continue.

        self.set_terminal_options()?;
        // Drop the previous screen so the screen gets fully redrawn after going to the foreground.
        self.frontend.lock().unwrap().previous_screen();
        Ok(())
    }

    pub fn components(&self) -> Vec<KindedComponent> {
        self.layout.components()
    }

    /// Returns true if the app should quit.
    pub fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        // Pass event to focused window
        let component = self.current_component();
        match event {
            Event::Resize(columns, rows) => {
                self.resize(Dimension {
                    height: rows as usize,
                    width: columns as usize,
                });
            }
            event => {
                let dispatches = component.borrow_mut().handle_event(&self.context, event);
                self.handle_dispatches_result(dispatches)
                    .unwrap_or_else(|e| {
                        self.show_global_info(Info::new("ERROR".to_string(), e.to_string()))
                    });
            }
        }

        Ok(false)
    }

    /// Return true if there's no more windows
    fn should_quit(&mut self) -> bool {
        self.layout.components().is_empty()
    }

    pub fn render(&mut self) -> Result<(), anyhow::Error> {
        let screen = self.get_screen()?;
        self.render_screen(screen)?;
        Ok(())
    }

    fn keyboard_layout_kind(&self) -> &KeyboardLayoutKind {
        self.context.keyboard_layout_kind()
    }

    pub fn get_screen(&mut self) -> Result<Screen, anyhow::Error> {
        // Recalculate layout before each render
        self.layout.recalculate_layout(&self.context);

        // Generate layout
        // Render every window
        let (windows, cursors): (Vec<_>, Vec<_>) = self
            .components()
            .into_iter()
            .map(|component| {
                let rectangle = component.component().borrow().rectangle().clone();
                let focused_component_id = self.layout.focused_component_id();
                let focused = component.component().borrow().id() == focused_component_id;
                let GetGridResult { grid, cursor } = component
                    .component()
                    .borrow_mut()
                    .get_grid(&self.context, focused);
                let cursor_position = 'cursor_calc: {
                    if !focused {
                        break 'cursor_calc None;
                    }

                    let Some(cursor) = cursor else {
                        break 'cursor_calc None;
                    };

                    let cursor_position = cursor.position();

                    // If cursor position is not in view
                    if cursor_position.line >= rectangle.dimension().height {
                        break 'cursor_calc None;
                    }

                    let calibrated_position = Position::new(
                        cursor_position.line + rectangle.origin.line,
                        cursor_position.column + rectangle.origin.column,
                    );

                    Some(cursor.set_position(calibrated_position))
                };
                let window = Window::new(grid, rectangle.clone());

                (window, cursor_position)
            })
            .unzip();
        let borders = self.layout.borders();
        let cursor = cursors.into_iter().find_map(|cursor| cursor);
        let screen = Screen::new(windows, borders, cursor, self.context.theme().ui.border);

        // Set the global title
        let global_title_windows = self
            .status_lines
            .iter()
            .enumerate()
            .map(|(index, status_line)| self.render_status_line(index, status_line))
            .collect_vec();
        let screen = global_title_windows
            .into_iter()
            .fold(screen, |screen, window| screen.add_window(window));

        Ok(screen)
    }

    fn render_status_line(&self, index: usize, status_line: &StatusLine) -> Window {
        let dimension = self.layout.terminal_dimension();
        let leading_padding = 1;
        let title = self.global_title.clone().unwrap_or_else(|| {
            let separator = "   ";
            let width = dimension
                .width
                .saturating_sub(leading_padding)
                .saturating_sub(1); // This is the extra space for rendering cursor at the last column
            render_flex_layout::render_flex_layout(
                width,
                separator,
                &status_line
                    .components
                    .iter()
                    .filter_map(|component| match component {
                        StatusLineComponent::Spacer => Some(FlexLayoutComponent::Spacer),
                        StatusLineComponent::LineColumn => self
                            .current_component()
                            .borrow()
                            .editor()
                            .get_cursor_position()
                            .ok()
                            .map(|position| {
                                FlexLayoutComponent::Text(format!(
                                    "{: >4}:{: <3}",
                                    position.line + 1,
                                    position.column + 1
                                ))
                            }),
                        StatusLineComponent::KiCharacter => {
                            Some(FlexLayoutComponent::Text("ⵣ".to_string()))
                        }
                        StatusLineComponent::CurrentWorkingDirectory => {
                            Some(FlexLayoutComponent::Text(
                                self.working_directory()
                                    .display_relative_to_home()
                                    .ok()
                                    .unwrap_or_else(|| self.working_directory().display_absolute()),
                            ))
                        }
                        StatusLineComponent::GitBranch => self
                            .current_branch()
                            .map(|branch| FlexLayoutComponent::Text(format!("⎇ {branch}"))),
                        StatusLineComponent::Mode => {
                            let mode = self
                                .context
                                .mode()
                                .map(|mode| mode.display())
                                .unwrap_or_else(|| {
                                    self.current_component().borrow().editor().display_mode()
                                });
                            Some(FlexLayoutComponent::Text(format!("{mode: <5}")))
                        }
                        StatusLineComponent::SelectionMode => Some(FlexLayoutComponent::Text(
                            self.current_component()
                                .borrow()
                                .editor()
                                .display_selection_mode(),
                        )),
                        StatusLineComponent::LastDispatch => self
                            .last_action_description
                            .clone()
                            .map(FlexLayoutComponent::Text),
                        StatusLineComponent::LastSearchString => self
                            .context
                            .get_prompt_history(PromptHistoryKey::Search)
                            .last()
                            .map(|search| FlexLayoutComponent::Text(format!("{search:?}"))),
                        StatusLineComponent::Help => {
                            let help_key = self
                                .context
                                .keyboard_layout_kind()
                                .translate_char_to_qwerty('/');
                            Some(FlexLayoutComponent::Text(format!("Help(Space+{help_key})")))
                        }
                        StatusLineComponent::KeyboardLayout => Some(FlexLayoutComponent::Text(
                            self.keyboard_layout_kind().display().to_string(),
                        )),
                        StatusLineComponent::Reveal => self
                            .current_component()
                            .borrow()
                            .editor()
                            .reveal()
                            .map(|split| {
                                match split {
                                    Reveal::CurrentSelectionMode => "÷SELS",
                                    Reveal::Cursor => "÷CURS",
                                    Reveal::Mark => "÷MARK",
                                }
                                .to_string()
                            })
                            .map(FlexLayoutComponent::Text),
                        StatusLineComponent::CurrentFileParentFolder => {
                            self.get_current_file_path().and_then(|path| {
                                Some(FlexLayoutComponent::Text({
                                    let path = path.parent().ok()??;
                                    path.display_relative_to(
                                        self.context.current_working_directory(),
                                    )
                                    .or_else(|_| path.display_relative_to_home())
                                    .unwrap_or_else(|_| path.display_absolute())
                                }))
                            })
                        }
                        StatusLineComponent::LspProgress => {
                            Some(FlexLayoutComponent::Text(self.context.lsp_progress()))
                        }
                        StatusLineComponent::Language => self
                            .current_component()
                            .borrow()
                            .editor()
                            .language()
                            .map(FlexLayoutComponent::Text),
                    })
                    .collect_vec(),
            )
        });
        let title = format!("{}{}", " ".repeat(leading_padding), title);
        let grid = Grid::new(Dimension {
            height: 1,
            width: dimension.width,
        })
        .render_content(
            &title,
            crate::grid::RenderContentLineNumber::NoLineNumber,
            Vec::new(),
            [LineUpdate {
                line_index: 0,
                style: self.context.theme().ui.global_title,
            }]
            .to_vec(),
            self.context.theme(),
            None,
            &[],
        );
        Window::new(
            grid,
            crate::rectangle::Rectangle {
                width: dimension.width,
                height: 1,
                origin: Position {
                    line: dimension.height + index,
                    column: 0,
                },
            },
        )
    }

    fn current_branch(&self) -> Option<String> {
        // Open the repository
        let repo = git2::Repository::open(self.working_directory().display_absolute()).ok()?;

        // Get the current branch
        let head = repo.head().ok()?;
        let branch = head.shorthand()?;
        Some(branch.to_string())
    }

    fn render_screen(&mut self, screen: Screen) -> Result<(), anyhow::Error> {
        let mut frontend = self.frontend.lock().unwrap();
        frontend.hide_cursor()?;
        let cursor = screen.cursor();
        frontend.render_screen(screen)?;
        if let Some(cursor) = cursor {
            frontend.show_cursor(&cursor)?;
        }

        Ok(())
    }

    fn handle_dispatches_result(
        &mut self,
        dispatches: anyhow::Result<Dispatches>,
    ) -> anyhow::Result<()> {
        self.handle_dispatches(dispatches?)
    }

    pub fn handle_dispatches(&mut self, dispatches: Dispatches) -> Result<(), anyhow::Error> {
        for dispatch in dispatches.into_vec() {
            self.handle_dispatch(dispatch)?;
        }
        Ok(())
    }

    fn get_request_params(&self) -> Option<RequestParams> {
        if self.current_component().borrow().type_id() != TypeId::of::<SuggestiveEditor>() {
            None
        } else {
            self.current_component()
                .borrow()
                .editor()
                .get_request_params()
        }
    }

    fn send_integration_event(&self, event: crate::integration_event::IntegrationEvent) {
        self.integration_event_sender.emit_event(event)
    }

    pub fn handle_dispatch(&mut self, dispatch: Dispatch) -> Result<(), anyhow::Error> {
        log::info!("App::handle_dispatch = {}", dispatch.variant_name());
        match dispatch {
            Dispatch::Suspend => {
                self.suspend()?;
            }
            Dispatch::CloseCurrentWindow => {
                self.close_current_window()?;
            }
            Dispatch::CloseCurrentWindowAndFocusParent => {
                self.close_current_window_and_focus_parent();
            }
            Dispatch::OpenSearchPrompt {
                scope,
                if_current_not_found,
            } => self.open_search_prompt(scope, if_current_not_found, None)?,
            Dispatch::OpenSearchPromptWithPriorChange {
                scope,
                if_current_not_found,
                prior_change,
            } => self.open_search_prompt_with_prior_change(
                scope,
                if_current_not_found,
                prior_change,
            )?,
            Dispatch::OpenPipeToShellPrompt => self.open_pipe_to_shell_prompt()?,
            Dispatch::OpenFile { path, owner, focus } => {
                self.open_file(&path, owner, true, focus)?;
            }
            Dispatch::OpenFileFromPathBuf { path, owner, focus } => {
                let canonicalized_path = path.try_into()?;
                self.open_file(&canonicalized_path, owner, true, focus)?;
            }
            Dispatch::OpenFilePicker(kind) => {
                self.open_file_picker(kind)?;
            }
            Dispatch::RequestCompletion => self.debounce_lsp_request_completion.call(()),
            Dispatch::RequestCompletionDebounced => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentCompletion(params),
                    )?;
                }
            }
            Dispatch::ResolveCompletionItem(completion_item) => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::CompletionItemResolve {
                            completion_item: Box::new(completion_item),
                            params,
                        },
                    )?
                }
            }
            Dispatch::RequestReferences {
                include_declaration,
                scope,
            } => {
                if let Some(params) = self.get_request_params() {
                    let params =
                        params
                            .set_kind(Some(scope))
                            .set_description(if include_declaration {
                                "References (include declaration)"
                            } else {
                                "References (exclude declaration)"
                            });
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentReferences {
                            params,
                            include_declaration,
                        },
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspReferences);
                }
            }
            Dispatch::RequestHover => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_description("Hover");
                    self.lsp_manager()
                        .send_message(params.path.clone(), FromEditor::TextDocumentHover(params))?;
                }

                self.send_integration_event(IntegrationEvent::RequestLspHover);
            }
            Dispatch::RequestDefinitions(scope) => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_kind(Some(scope)).set_description("Definitions");
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentDefinition(params),
                    )?;

                    self.send_integration_event(IntegrationEvent::RequestLspDefinition);
                }
            }
            Dispatch::RequestDeclarations(scope) => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_kind(Some(scope)).set_description("Declarations");
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentDeclaration(params),
                    )?;

                    self.send_integration_event(IntegrationEvent::RequestLspDeclaration);
                }
            }
            Dispatch::RequestImplementations(scope) => {
                if let Some(params) = self.get_request_params() {
                    let params = params
                        .set_kind(Some(scope))
                        .set_description("Implementations");
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentImplementation(params),
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspImplementation);
                }
            }
            Dispatch::RequestTypeDefinitions(scope) => {
                if let Some(params) = self.get_request_params() {
                    let params = params
                        .set_kind(Some(scope))
                        .set_description("Type Definitions");
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentTypeDefinition(params),
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspTypeDefinition);
                }
            }
            Dispatch::RequestDocumentSymbols => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_description("Document Symbols");
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentDocumentSymbol(params),
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspDocumentSymbols);
                }
            }
            Dispatch::RequestWorkspaceSymbols { query, path } => {
                self.lsp_manager().send_message(
                    path.clone(),
                    FromEditor::WorkspaceSymbol {
                        context: ResponseContext::default(),
                        query,
                    },
                )?;
            }
            Dispatch::PrepareRename => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentPrepareRename(params),
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspRename);
                }
            }
            Dispatch::RenameSymbol { new_name } => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentRename { params, new_name },
                    )?;
                }
            }
            Dispatch::RequestCodeAction { diagnostics } => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentCodeAction {
                            params,
                            diagnostics,
                        },
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspCodeAction);
                }
            }
            Dispatch::RequestSignatureHelp => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentSignatureHelp(params),
                    )?;
                }
            }
            Dispatch::DocumentDidChange {
                path,
                content,
                language,
                component_id,
                batch_id,
            } => {
                if let Some(language) = language {
                    self.request_syntax_highlight(
                        component_id,
                        batch_id,
                        language,
                        content.clone(),
                    )?;
                }
                if let Some(path) = path.clone() {
                    self.lsp_manager().send_message(
                        path.clone(),
                        FromEditor::TextDocumentDidChange {
                            content,
                            file_path: path,
                            version: 2,
                        },
                    )?;
                }
            }
            Dispatch::DocumentDidSave { path } => {
                // Emit an integration event for buffer save
                // Find the component that has this path
                for component in self.layout.components() {
                    // Store the component reference to extend its lifetime
                    let component_rc = component.component();
                    let component_ref = component_rc.borrow();
                    if let Some(component_path) = component_ref.path() {
                        if component_path == path {
                            self.integration_event_sender.emit_event(
                                crate::integration_event::IntegrationEvent::BufferSaved {
                                    path: path.clone(),
                                },
                            );
                            break;
                        }
                    }
                }

                self.lsp_manager().send_message(
                    path.clone(),
                    FromEditor::TextDocumentDidSave { file_path: path },
                )?;
            }
            Dispatch::SetQuickfixList(r#type) => {
                self.set_quickfix_list_type(Default::default(), r#type)?;
            }
            Dispatch::GotoQuickfixListItem(movement) => self.goto_quickfix_list_item(movement)?,
            Dispatch::ApplyWorkspaceEdit(workspace_edit) => {
                self.apply_workspace_edit(workspace_edit)?;
            }
            Dispatch::ShowKeymapLegend(keymap_legend_config) => {
                self.show_keymap_legend(keymap_legend_config, None)
            }
            Dispatch::ShowKeymapLegendWithReleaseKey(keymap_legend_config, release_key) => {
                self.show_keymap_legend(keymap_legend_config, Some(release_key))
            }
            #[cfg(test)]
            Dispatch::Custom(_) => unreachable!(),
            Dispatch::RemainOnlyCurrentComponent => self.layout.remain_only_current_component(),
            Dispatch::ToEditor(dispatch_editor) => self.handle_dispatch_editor(dispatch_editor)?,
            Dispatch::GotoLocation(location) => self.go_to_location(&location, true)?,
            Dispatch::OpenMoveToIndexPrompt(prior_change) => {
                self.open_move_to_index_prompt(prior_change)?
            }
            Dispatch::QuitAll => self.quit_all()?,
            Dispatch::RevealInExplorer(path) => self.reveal_path_in_explorer(&path)?,
            Dispatch::OpenMovePathsPrompt => self.open_move_paths_prompt()?,
            Dispatch::OpenDuplicateFilePrompt => self.open_copy_file_prompt()?,
            Dispatch::OpenAddPathPrompt => self.open_add_path_prompt()?,
            Dispatch::OpenDeletePathsPrompt => self.open_delete_file_prompt()?,
            Dispatch::DeletePaths(paths) => self.delete_paths(paths)?,
            Dispatch::Null => {
                // do nothing
            }
            Dispatch::MovePaths {
                sources,
                destinations,
            } => self.move_paths(sources, destinations)?,
            Dispatch::CopyFile { from, to } => self.copy_file(from, to)?,
            Dispatch::AddPath(path) => self.add_path(path)?,
            Dispatch::RefreshFileExplorer => self.layout.refresh_file_explorer(&self.context)?,
            Dispatch::SetClipboardContent {
                copied_texts: contents,
            } => {
                self.context
                    .set_clipboard_content(contents.clone())
                    .or_else(|_| {
                        let mut frontend = self.frontend.lock().unwrap();
                        frontend.set_clipboard_with_osc52(&contents.to_text())
                    })?;
            }
            Dispatch::SetGlobalMode(mode) => self.set_global_mode(mode)?,
            #[cfg(test)]
            Dispatch::HandleKeyEvent(key_event) => {
                self.handle_event(Event::Key(key_event))?;
            }
            #[cfg(test)]
            Dispatch::HandleEvent(event) => {
                self.handle_event(event)?;
            }
            Dispatch::GetRepoGitHunks(diff_mode) => self.get_repo_git_hunks(diff_mode)?,
            Dispatch::SaveAll => self.save_all()?,
            #[cfg(test)]
            Dispatch::TerminalDimensionChanged(dimension) => self.resize(dimension),
            #[cfg(test)]
            Dispatch::SetGlobalTitle(title) => self.set_global_title(title),
            Dispatch::LspExecuteCommand { command } => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager().send_message(
                        params.path.clone(),
                        FromEditor::WorkspaceExecuteCommand { params, command },
                    )?
                };
            }
            Dispatch::UpdateLocalSearchConfig {
                update,
                scope,
                if_current_not_found,
                run_search_after_config_updated,
                component_id,
            } => self.update_local_search_config(
                update,
                scope,
                if_current_not_found,
                run_search_after_config_updated,
                component_id,
            )?,
            Dispatch::UpdateGlobalSearchConfig { update } => {
                self.update_global_search_config(update)?;
            }
            Dispatch::Replace { scope } => match scope {
                Scope::Local => self.handle_dispatch_editor(ReplacePattern {
                    config: self.context.local_search_config(Scope::Local).clone(),
                })?,
                Scope::Global => self.global_replace()?,
            },
            #[cfg(test)]
            Dispatch::HandleLspNotification(notification) => {
                self.handle_lsp_notification(notification)?
            }
            Dispatch::SetTheme(theme) => {
                self.context.set_theme(theme.clone());
            }
            Dispatch::SetThemeFromDescriptor(theme_descriptor) => {
                self.context.set_theme(theme_descriptor.to_theme());
            }
            #[cfg(test)]
            Dispatch::HandleKeyEvents(key_events) => self.handle_key_events(key_events)?,
            Dispatch::CloseDropdown => self.layout.close_dropdown(),
            Dispatch::CloseEditorInfo => self.layout.close_editor_info(),
            Dispatch::CloseGlobalInfo => self.layout.close_global_info(),
            Dispatch::RenderDropdown { render } => {
                if let Some(dropdown) = self.layout.open_dropdown(&self.context) {
                    self.render_dropdown(dropdown, render)?;
                }
            }
            #[cfg(test)]
            Dispatch::OpenPrompt { config } => self.open_prompt(config)?,
            Dispatch::ShowEditorInfo(info) => self.show_editor_info(info)?,
            Dispatch::ReceiveCodeActions(code_actions) => {
                self.open_code_actions_picker(code_actions)?;
            }
            Dispatch::OtherWindow => self.layout.cycle_window(),
            Dispatch::CycleMarkedFile(movement) => self.cycle_marked_file(movement)?,
            Dispatch::PushPromptHistory { key, line } => self.push_history_prompt(key, line),
            Dispatch::OpenThemePrompt => self.open_theme_picker()?,
            Dispatch::SetLastNonContiguousSelectionMode(selection_mode) => self
                .context
                .set_last_non_contiguous_selection_mode(selection_mode),
            Dispatch::UseLastNonContiguousSelectionMode(if_current_not_found) => {
                self.use_last_non_contiguous_selection_mode(if_current_not_found)?
            }
            Dispatch::SetLastActionDescription {
                long_description: description,
                short_description,
            } => {
                self.last_action_description = Some(description);
                self.last_action_short_description = short_description
            }
            Dispatch::OpenFilterSelectionsPrompt { maintain } => {
                self.open_filter_selections_prompt(maintain)?
            }
            Dispatch::MoveToCompletionItem(direction) => self.handle_dispatch_suggestive_editor(
                DispatchSuggestiveEditor::MoveToCompletionItem(direction),
            )?,
            Dispatch::SelectCompletionItem => self.handle_dispatch_suggestive_editor(
                DispatchSuggestiveEditor::SelectCompletionItem,
            )?,
            Dispatch::SetKeyboardLayoutKind(keyboard_layout_kind) => {
                self.context.set_keyboard_layout_kind(keyboard_layout_kind);
                self.keyboard_layout_changed();
            }
            Dispatch::OpenKeyboardLayoutPrompt => self.open_keyboard_layout_picker()?,
            Dispatch::NavigateForward => self.navigate_forward()?,
            Dispatch::NavigateBack => self.navigate_back()?,
            Dispatch::MarkFileAndToggleMark => self.mark_file_and_toggle_mark()?,
            Dispatch::ToggleFileMark => self.toggle_file_mark()?,
            Dispatch::ToHostApp(to_host_app) => self.handle_to_host_app(to_host_app)?,
            Dispatch::FromHostApp(from_host_app) => self.handle_from_host_app(from_host_app)?,
            Dispatch::OpenSurroundXmlPrompt => self.open_surround_xml_prompt()?,
            Dispatch::OpenSearchPromptWithCurrentSelection {
                scope,
                prior_change,
            } => self.open_search_prompt_with_current_selection(scope, prior_change)?,
            Dispatch::ShowGlobalInfo(info) => self.show_global_info(info),
            Dispatch::DropdownFilterUpdated(filter) => {
                self.handle_dropdown_filter_updated(filter)?
            }
            #[cfg(test)]
            Dispatch::SetSystemClipboardHtml { html, alt_text } => {
                self.set_system_clipboard_html(html, alt_text)?
            }
            Dispatch::AddQuickfixListEntries(locations) => {
                self.add_quickfix_list_entries(locations)?
            }
            Dispatch::AppliedEdits { path, edits } => self.handle_applied_edits(path, edits),
            Dispatch::ExecuteLeaderKey(key) => self.execute_leader_key(key)?,
            Dispatch::ShowBufferSaveConflictPrompt {
                path,
                content_editor,
                content_filesystem,
            } => {
                self.show_buffer_save_conflict_prompt(&path, content_editor, content_filesystem)?
            }
            Dispatch::OpenWorkspaceSymbolsPrompt => self.open_workspace_symbols_picker()?,
            Dispatch::GetAndHandlePromptOnChangeDispatches => {
                self.get_and_handle_prompt_on_change_dispatches()?
            }
            Dispatch::SetIncrementalSearchConfig {
                config,
                component_id,
            } => self.set_incremental_search_config(config, component_id),
            Dispatch::UpdateCurrentComponentTitle(title) => {
                self.update_current_component_title(title)
            }
            Dispatch::SaveMarks { path, marks } => self.context.save_marks(path, marks),
            Dispatch::ToSuggestiveEditor(dispatch) => {
                self.handle_dispatch_suggestive_editor(dispatch)?;
            }
            Dispatch::OpenAndMarkFiles(paths) => self.open_and_mark_files(paths)?,
            Dispatch::ToggleOrOpenPaths => self.toggle_or_open_paths()?,
            Dispatch::ChangeWorkingDirectory(path) => self.change_working_directory(path)?,
            Dispatch::AddClipboardHistory(copied_texts) => {
                self.context.add_clipboard_history(copied_texts)
            }
            Dispatch::OpenChangeWorkingDirectoryPrompt => {
                self.open_change_working_directory_prompt()?
            }
        }
        Ok(())
    }

    pub fn get_editor_by_file_path(
        &self,
        path: &CanonicalizedPath,
    ) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        self.layout.get_existing_editor(path)
    }

    pub fn current_component(&self) -> Rc<RefCell<dyn Component>> {
        self.layout.get_current_component()
    }

    fn close_current_window(&mut self) -> anyhow::Result<()> {
        if let Some(removed_path) = self.layout.close_current_window(&self.context) {
            self.send_file_watcher_input(FileWatcherInput::SyncOpenedPaths(
                self.layout.get_opened_files(),
            ));
            if let Some(path) = self.context.unmark_path(removed_path).cloned() {
                self.open_file(&path, BufferOwner::User, true, true)?;
            }
        }
        Ok(())
    }

    fn local_search(
        &mut self,
        if_current_not_found: IfCurrentNotFound,
        component_id: Option<ComponentId>,
    ) -> anyhow::Result<()> {
        let config = self.context.local_search_config(Scope::Local);
        let search = config.search();
        if !search.is_empty() {
            self.handle_dispatch_editor_custom(
                SetSelectionMode(
                    if_current_not_found,
                    SelectionMode::Find {
                        search: Search {
                            mode: config.mode,
                            search,
                        },
                    },
                ),
                component_id
                    .and_then(|component_id| self.get_component_by_id(component_id))
                    .unwrap_or_else(|| self.current_component()),
            )?;
        }

        Ok(())
    }

    fn resize(&mut self, dimension: Dimension) {
        self.layout.set_terminal_dimension(
            dimension.decrement_height(self.global_title_bar_height()),
            &self.context,
        );
    }

    fn open_move_to_index_prompt(
        &mut self,
        prior_change: Option<PriorChange>,
    ) -> anyhow::Result<()> {
        self.current_component()
            .borrow_mut()
            .editor_mut()
            .handle_prior_change(prior_change);
        self.open_prompt(PromptConfig::new(
            "Move to index".to_string(),
            PromptOnEnter::ParseCurrentLine {
                parser: DispatchParser::MoveSelectionByIndex,
                history_key: PromptHistoryKey::MoveToIndex,
                current_line: None,
                suggested_items: Default::default(),
            },
        ))
    }

    fn open_rename_prompt(&mut self, current_name: Option<String>) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig::new(
            "Rename Symbol".to_string(),
            PromptOnEnter::ParseCurrentLine {
                parser: DispatchParser::RenameSymbol,
                history_key: PromptHistoryKey::Rename,
                current_line: current_name,
                suggested_items: Default::default(),
            },
        ))
    }

    fn open_surround_xml_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig::new(
            "Surround selection with XML tag (can be empty for React Fragment)".to_string(),
            PromptOnEnter::ParseCurrentLine {
                parser: DispatchParser::SurroundXmlTag,
                history_key: PromptHistoryKey::SurroundXmlTag,
                current_line: None,
                suggested_items: Default::default(),
            },
        ))
    }

    fn open_search_prompt_with_current_selection(
        &mut self,
        scope: Scope,
        prior_change: Option<PriorChange>,
    ) -> anyhow::Result<()> {
        self.current_component()
            .borrow_mut()
            .editor_mut()
            .handle_prior_change(prior_change);
        let current_line = self
            .current_component()
            .borrow()
            .editor()
            .current_primary_selection()?;
        self.open_search_prompt(scope, IfCurrentNotFound::LookForward, Some(current_line))
    }

    fn open_search_prompt(
        &mut self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        current_line: Option<String>,
    ) -> anyhow::Result<()> {
        self.open_prompt({
            PromptConfig::new(
                format!("{scope:?} search",),
                PromptOnEnter::ParseCurrentLine {
                    parser: DispatchParser::UpdateLocalSearchConfigSearch {
                        scope,
                        if_current_not_found,
                        run_search_after_config_updated: true,
                    },
                    history_key: PromptHistoryKey::Search,
                    current_line,
                    suggested_items: self.words(),
                },
            )
            .set_on_change(match scope {
                Scope::Local => Some({
                    let component_id = self.current_component().borrow().id();
                    PromptOnChangeDispatch::SetIncrementalSearchConfig { component_id }
                }),
                Scope::Global => None,
            })
            .set_on_cancelled(Some(Dispatches::one(Dispatch::ToEditor(
                DispatchEditor::ClearIncrementalSearchMatches,
            ))))
        })
    }

    fn get_file_explorer_current_path(&mut self) -> anyhow::Result<Option<CanonicalizedPath>> {
        self.current_component()
            .borrow_mut()
            .as_any_mut()
            .downcast_mut::<FileExplorer>()
            .and_then(|file_explorer| file_explorer.get_current_path().transpose())
            .transpose()
    }

    fn get_file_explorer_selected_paths(&mut self) -> anyhow::Result<Vec<CanonicalizedPath>> {
        self.current_component()
            .borrow_mut()
            .as_any_mut()
            .downcast_mut::<FileExplorer>()
            .ok_or_else(|| {
                anyhow::anyhow!("Unable to downcast current component to `FileExplorer`")
            })?
            .get_selected_paths()
    }

    fn open_delete_file_prompt(&mut self) -> anyhow::Result<()> {
        let selected_paths = self.get_file_explorer_selected_paths()?;
        if let Some(selected_paths) = NonEmpty::from_vec(selected_paths) {
            let formatted_paths = selected_paths
                .iter()
                .map(|path| {
                    format!(
                        "'{}'",
                        path.try_display_relative_to(self.context.current_working_directory())
                    )
                })
                .join(", ");
            self.open_yes_no_prompt(YesNoPrompt {
                title: format!("Delete \"{formatted_paths}\"?"),
                yes: Box::new(Dispatch::DeletePaths(selected_paths.clone())),
            })
        } else {
            self.show_global_info(Info::new(
                "Delete file error".to_owned(),
                "No paths are selected.".to_owned(),
            ));
            Ok(())
        }
    }

    fn open_add_path_prompt(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.get_file_explorer_current_path()? {
            self.open_prompt(PromptConfig::new(
                "Add path".to_string(),
                PromptOnEnter::ParseCurrentLine {
                    parser: DispatchParser::AddPath,
                    history_key: PromptHistoryKey::AddPath,
                    current_line: Some(path.display_absolute()),
                    suggested_items: Default::default(),
                },
            ))
        } else {
            Ok(())
        }
    }

    fn open_move_paths_prompt(&mut self) -> anyhow::Result<()> {
        let paths = self.get_file_explorer_selected_paths()?;
        if let Some(paths) = NonEmpty::from_vec(paths) {
            let initial_lines = paths
                .as_ref()
                .map(|path| path.display_absolute())
                .into_iter()
                .collect_vec();
            self.open_prompt(PromptConfig::new(
                "Move paths".to_string(),
                PromptOnEnter::ParseWholeBuffer {
                    parser: DispatchParser::MovePaths { sources: paths },
                    initial_lines,
                },
            ))
        } else {
            Ok(())
        }
    }

    fn open_copy_file_prompt(&mut self) -> anyhow::Result<()> {
        let path = self.get_file_explorer_current_path()?;
        if let Some(path) = path {
            self.open_prompt(PromptConfig::new(
                format!("Duplicate '{}' to", path.display_absolute()),
                PromptOnEnter::ParseCurrentLine {
                    parser: DispatchParser::CopyFile { from: path.clone() },
                    history_key: PromptHistoryKey::CopyFile,
                    current_line: Some(path.display_absolute()),
                    suggested_items: Default::default(),
                },
            ))
        } else {
            Ok(())
        }
    }

    fn open_symbol_picker(&mut self, symbols: Symbols) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig::new(
            "Symbols".to_string(),
            PromptOnEnter::SelectsFirstMatchingItem {
                items: PromptItems::Precomputed(
                    symbols
                        .symbols
                        .clone()
                        .into_iter()
                        .map(|symbol| symbol.into())
                        .collect_vec(),
                ),
            },
        ))
    }

    fn open_file_picker(&mut self, kind: FilePickerKind) -> anyhow::Result<()> {
        let working_directory = self.working_directory().clone();
        let items = match kind {
            FilePickerKind::NonGitIgnored => PromptItems::BackgroundTask {
                task: PromptItemsBackgroundTask::NonGitIgnoredFiles { working_directory },
                on_nucleo_tick_debounced: {
                    let sender = self.sender.clone();
                    Callback::new(Arc::new(move |_| {
                        let _ = sender.send(AppMessage::NucleoTickDebounced);
                    }))
                },
            },
            FilePickerKind::GitStatus(diff_mode) => PromptItems::Precomputed(
                git::GitRepo::try_from(self.working_directory())?
                    .diff_entries(diff_mode)?
                    .into_iter()
                    .map(|entry| {
                        DropdownItem::from_path_buf(
                            &working_directory,
                            entry.new_path().into_path_buf(),
                        )
                    })
                    .collect_vec(),
            ),
            FilePickerKind::Opened => PromptItems::Precomputed(
                self.layout
                    .get_opened_files()
                    .into_iter()
                    .map(|path| {
                        DropdownItem::from_path_buf(&working_directory, path.into_path_buf())
                    })
                    .collect_vec(),
            ),
        };
        self.open_prompt(PromptConfig::new(
            format!("Open file: {}", kind.display()),
            PromptOnEnter::SelectsFirstMatchingItem { items },
        ))
    }

    fn open_file(
        &mut self,
        path: &CanonicalizedPath,
        owner: BufferOwner,
        store_history: bool,
        focus: bool,
    ) -> anyhow::Result<Rc<RefCell<SuggestiveEditor>>> {
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "The path {:?} does not exist.",
                path.try_display_relative_to(self.context.current_working_directory()),
            ));
        }
        if !path.is_file() {
            return Err(anyhow::anyhow!(
                "The path {:?} is not a file.",
                path.try_display_relative_to(self.context.current_working_directory()),
            ));
        }

        if store_history {
            self.push_current_location_into_navigation_history(true);
        }

        // Check if the file is opened before so that we won't notify the LSP twice
        if let Some(matching_editor) = self.layout.open_file(path, focus) {
            return Ok(matching_editor);
        }

        let mut buffer = Buffer::from_path(path, true)?;
        buffer.set_owner(owner);

        let language = buffer.language();
        let content = buffer.content();
        let batch_id = buffer.batch_id().clone();
        let buffer = Rc::new(RefCell::new(buffer));
        let editor = SuggestiveEditor::from_buffer(buffer, SuggestiveEditorFilter::CurrentWord);
        let component_id = editor.id();
        let component = Rc::new(RefCell::new(editor));

        self.layout.add_suggestive_editor(component.clone());

        if focus {
            self.layout
                .replace_and_focus_current_suggestive_editor(component.clone());
        }
        if let Some(language) = language {
            self.request_syntax_highlight(component_id, batch_id, language, content)?;
        }
        if self.enable_lsp {
            self.lsp_manager().open_file(path.clone())?;
        }

        self.send_file_watcher_input(FileWatcherInput::SyncOpenedPaths(
            self.layout.get_opened_files(),
        ));
        Ok(component)
    }

    pub fn handle_lsp_notification(&mut self, notification: LspNotification) -> anyhow::Result<()> {
        match notification {
            LspNotification::Hover(hover) => self.show_editor_info(Info::new(
                "Hover Info".to_string(),
                hover.contents.join("\n\n"),
            )),
            LspNotification::Definition(context, response) => {
                match response {
                    GotoDefinitionResponse::Single(location) => {
                        self.go_to_location(&location, true)?
                    }
                    GotoDefinitionResponse::Multiple(locations) => {
                        if locations.is_empty() {
                            self.show_global_info(Info::new(
                                "Goto definition info".to_string(),
                                "No definitions found".to_string(),
                            ));
                        } else {
                            self.set_quickfix_list_type(
                                context,
                                QuickfixListType::Items(
                                    locations.into_iter().map(QuickfixListItem::from).collect(),
                                ),
                            )?;
                        }
                    }
                }

                Ok(())
            }
            LspNotification::References(context, locations) => self.set_quickfix_list_type(
                context,
                QuickfixListType::Items(
                    locations.into_iter().map(QuickfixListItem::from).collect(),
                ),
            ),
            LspNotification::Completion(_context, completion) => {
                self.handle_dispatch_suggestive_editor(DispatchSuggestiveEditor::Completion(
                    completion,
                ))?;

                Ok(())
            }
            LspNotification::Initialized(language) => {
                // Need to notify LSP that the file is opened
                let opened_documents = self
                    .layout
                    .buffers()
                    .into_iter()
                    .filter_map(|buffer| {
                        if buffer.borrow().language()? == *language {
                            buffer.borrow().path()
                        } else {
                            None
                        }
                    })
                    .collect_vec();
                self.lsp_manager().initialized(*language, opened_documents);
                Ok(())
            }
            LspNotification::PublishDiagnostics(params) => {
                self.update_diagnostics(
                    params
                        .uri
                        .to_file_path()
                        .map_err(|err| {
                            anyhow::anyhow!("Couldn't convert URI to file path: {:?}", err)
                        })?
                        .try_into()?,
                    params.diagnostics,
                )?;
                Ok(())
            }
            LspNotification::PrepareRenameResponse(response) => {
                let editor = self.current_component();

                let current_name = {
                    let editor = editor.borrow();
                    let buffer = editor.editor().buffer();
                    response
                        .range
                        .map(|range| {
                            let range = buffer.position_to_char(range.start)?
                                ..buffer.position_to_char(range.end)?;
                            buffer.slice(&range.into())
                        })
                        .transpose()
                        .unwrap_or_default()
                        .map(|rope| rope.to_string())
                };
                self.open_rename_prompt(current_name)?;

                Ok(())
            }
            LspNotification::Error(error) => {
                self.show_global_info(Info::new("LSP Error".to_string(), error));
                Ok(())
            }
            LspNotification::WorkspaceEdit(workspace_edit) => {
                self.apply_workspace_edit(workspace_edit)
            }
            LspNotification::CodeAction(code_actions) => {
                self.handle_dispatch(Dispatch::ReceiveCodeActions(code_actions))?;
                Ok(())
            }
            LspNotification::SignatureHelp(signature_help) => {
                self.handle_signature_help(signature_help)?;
                Ok(())
            }
            LspNotification::DocumentSymbols(symbols) => {
                self.open_symbol_picker(symbols)?;
                Ok(())
            }
            LspNotification::CompletionItemResolve(completion_item) => {
                self.update_current_completion_item((*completion_item).into())
            }
            LspNotification::WorkspaceSymbols(symbols) => self.handle_workspace_symbols(symbols),
            LspNotification::Progress { message } => {
                self.context.update_lsp_progress(message);
                Ok(())
            }
        }
    }

    pub fn update_diagnostics(
        &mut self,
        path: CanonicalizedPath,
        diagnostics: Vec<lsp_types::Diagnostic>,
    ) -> anyhow::Result<()> {
        let component = self.open_file(&path, BufferOwner::System, false, false)?;

        component
            .borrow_mut()
            .editor_mut()
            .buffer_mut()
            .set_diagnostics(diagnostics);
        Ok(())
    }

    fn goto_quickfix_list_item(&mut self, movement: Movement) -> anyhow::Result<()> {
        if let Some((current_item_index, dispatches)) =
            self.context.get_quickfix_list_item(movement)
        {
            self.context
                .set_quickfix_list_current_item_index(current_item_index);
            self.handle_dispatches(dispatches)?;
            self.render_quickfix_list()?;
        } else {
            log::info!("No current item found")
        }
        Ok(())
    }

    fn show_global_info(&mut self, info: Info) {
        if self.is_running_as_embedded() {
            self.integration_event_sender.emit_event(
                crate::integration_event::IntegrationEvent::ShowInfo {
                    info: Some(info.display()),
                },
            );
        }
        self.layout
            .show_global_info(info, &self.context)
            .unwrap_or_else(|err| {
                log::error!("Error showing info: {err:?}");
            });
    }

    fn go_to_location(
        &mut self,
        location: &Location,
        store_history: bool,
    ) -> Result<(), anyhow::Error> {
        let component = self.open_file(&location.path, BufferOwner::System, store_history, true)?;

        if self.is_running_as_embedded() {
            // Emit an integration event for selection change
            let component_ref = component.borrow();
            let component_id = component_ref.id();

            // Create a selection at the location position
            // We'll let the editor.set_position_range call below handle the actual selection
            // Just emit a simple empty selection at the start position for now
            let selection = crate::selection::Selection::new(location.range);

            // Emit a selection changed event
            self.integration_event_sender.emit_event(
                crate::integration_event::IntegrationEvent::SelectionChanged {
                    component_id,
                    selections: vec![selection],
                },
            );
        }

        let dispatches = component
            .borrow_mut()
            .editor_mut()
            .set_char_index_range(location.range, &self.context)?;
        self.handle_dispatches(dispatches)?;
        Ok(())
    }

    fn set_quickfix_list_type(
        &mut self,
        context: ResponseContext,
        r#type: QuickfixListType,
    ) -> anyhow::Result<()> {
        let title = context.description.unwrap_or_default();
        self.context.set_mode(Some(GlobalMode::QuickfixListItem));

        let source = match r#type {
            QuickfixListType::Diagnostic(severity_range) => {
                QuickfixListSource::Diagnostic(severity_range)
            }
            QuickfixListType::Items(items) => QuickfixListSource::Custom(items),
            QuickfixListType::Mark => QuickfixListSource::Mark,
        };

        let items = self.layout.get_quickfix_list_items(&source, &self.context);

        let go_to_first_quickfix = !items.is_empty();

        self.context.set_quickfix_list_items(&title, items);

        match context.scope {
            None | Some(Scope::Global) => {
                if go_to_first_quickfix {
                    self.goto_quickfix_list_item(Movement::Current(
                        IfCurrentNotFound::LookForward,
                    ))?;
                }
                Ok(())
            }
            Some(Scope::Local) => self.handle_dispatch(Dispatch::ToEditor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::LocalQuickfix { title },
            ))),
        }
    }

    fn apply_workspace_edit(&mut self, workspace_edit: WorkspaceEdit) -> Result<(), anyhow::Error> {
        // TODO: should we wrap this in a transaction so that if one of the edit/operation fails, the whole transaction fails?
        // Such that it won't leave the workspace in an half-edited messed up state
        for edit in workspace_edit.edits {
            let component = self.open_file(&edit.path, BufferOwner::System, false, false)?;
            let dispatches = component
                .borrow_mut()
                .editor_mut()
                .apply_positional_edits(edit.edits, &self.context)?;

            self.handle_dispatches(dispatches)?;

            let dispatches = component.borrow_mut().editor_mut().save(&self.context)?;

            self.handle_dispatches(dispatches)?;
        }
        use crate::lsp::workspace_edit::ResourceOperation;
        for operation in workspace_edit.resource_operations {
            match operation {
                ResourceOperation::Create(path) => self.add_path(path)?,
                ResourceOperation::Rename { old, new } => self.move_path(old, new)?,
                ResourceOperation::Delete(path) => self.delete_paths(NonEmpty::new(path))?,
            }
        }
        Ok(())
    }

    fn show_keymap_legend(
        &mut self,
        keymap_legend_config: KeymapLegendConfig,
        release_key: Option<ReleaseKey>,
    ) {
        if self.is_running_as_embedded() {
            let title = keymap_legend_config.title.clone();
            let body = keymap_legend_config.display(
                usize::MAX,
                &KeymapDisplayOption {
                    show_alt: false,
                    show_shift: true,
                },
            );
            self.integration_event_sender
                .emit_event(IntegrationEvent::ShowInfo {
                    info: Some(format!("{title}\n\n{body}")),
                });
        }
        self.layout
            .show_keymap_legend(keymap_legend_config, &self.context, release_key);
        self.layout.recalculate_layout(&self.context);
    }

    fn global_replace(&mut self) -> anyhow::Result<()> {
        let working_directory = self.working_directory().clone();
        let global_search_config = self.context.global_search_config();
        let walk_builder_config = WalkBuilderConfig {
            root: working_directory.clone().into(),
            include: global_search_config.include_glob(),
            exclude: global_search_config.exclude_glob(),
        };
        let config = self.context.global_search_config().local_config();
        let (dispatches, affected_paths) =
            list::grep::replace(walk_builder_config, config.clone())?;
        self.handle_dispatches(dispatches)?;
        let dispatches = self.layout.reload_buffers(affected_paths)?;
        self.handle_dispatches(dispatches)
    }

    fn global_search(&mut self) -> anyhow::Result<()> {
        let working_directory = self.working_directory().clone();

        let global_search_config = self.context.global_search_config();
        let walk_builder_config = WalkBuilderConfig {
            root: working_directory.clone().into(),
            include: global_search_config.include_glob(),
            exclude: global_search_config.exclude_glob(),
        };
        let config = global_search_config.local_config();
        if config.search().is_empty() {
            return Ok(());
        }
        let sender = self.sender.clone();
        let limit = 10000;
        let send_matches = Arc::new(move |result: crate::thread::BatchResult<Match>| {
            SendResult::from(
                sender.send(AppMessage::ExternalDispatch(Box::new(match result {
                    crate::thread::BatchResult::Items(matches) => {
                        Dispatch::AddQuickfixListEntries(matches)
                    }
                    crate::thread::BatchResult::LimitReached => {
                        Dispatch::ShowGlobalInfo(Info::new(
                            "Search Halted".to_string(),
                            format!("The search has more than {limit} matches and is thus halted to avoid performance issues."),
                        ))
                    }
                }))),
            )
        });
        let send_match = crate::thread::batch(send_matches, Duration::from_millis(16), limit); // Around 30 ticks per second

        // TODO: we need to create a new sender for each global search, so that it can be cancelled, but when?
        // Is it when the quickfix list is closed?
        match config.mode {
            LocalSearchConfigMode::Regex(regex) => {
                list::grep::run(&config.search(), walk_builder_config, regex, send_match)?;
            }
            LocalSearchConfigMode::AstGrep => {
                list::ast_grep::run(config.search().clone(), walk_builder_config, send_match)?;
            }
            LocalSearchConfigMode::NamingConventionAgnostic => {
                list::naming_convention_agnostic::run(
                    config.search().clone(),
                    walk_builder_config,
                    send_match,
                )?;
            }
        };
        self.set_quickfix_list_type(
            ResponseContext::default().set_description("Global search"),
            // We start with an empty quickfix list, as the result will come later
            // due to the asynchronity
            QuickfixListType::Items(Vec::new()),
        )?;
        Ok(())
    }

    pub fn quit_all(&self) -> Result<(), anyhow::Error> {
        Ok(self.sender.send(AppMessage::QuitAll)?)
    }

    pub fn sender(&self) -> Sender<AppMessage> {
        self.sender.clone()
    }

    fn save_all(&self) -> anyhow::Result<()> {
        self.layout.save_all(&self.context)
    }

    fn open_yes_no_prompt(&mut self, prompt: YesNoPrompt) -> anyhow::Result<()> {
        self.handle_dispatch(Dispatch::ShowKeymapLegend(KeymapLegendConfig {
            title: prompt.title.to_string(),
            keymap: Keymap::new(&[
                Keybinding::new("d", "Yes".to_string(), *prompt.yes),
                Keybinding::new("k", "No".to_string(), Dispatch::Null),
            ]),
        }))
    }

    fn delete_paths(&mut self, paths: NonEmpty<CanonicalizedPath>) -> anyhow::Result<()> {
        for path in paths {
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
            self.layout.remove_suggestive_editor(&path);
        }
        self.layout.refresh_file_explorer(&self.context)?;
        Ok(())
    }

    fn move_paths(
        &mut self,
        sources: NonEmpty<CanonicalizedPath>,
        destinations: NonEmpty<PathBuf>,
    ) -> anyhow::Result<()> {
        if sources.len() != destinations.len() {
            Err(anyhow::anyhow!(
                "Expected destination paths to have the length of {}, but got {}",
                sources.len(),
                destinations.len()
            ))
        } else {
            for (source, destination) in sources.into_iter().zip(destinations) {
                self.move_path(source, destination)?;
            }
            Ok(())
        }
    }

    fn move_path(&mut self, from: CanonicalizedPath, to: PathBuf) -> anyhow::Result<()> {
        use std::fs;
        self.add_path_parent(&to)?;
        fs::rename(from.clone(), to.clone())?;
        self.layout.refresh_file_explorer(&self.context)?;
        let to = to.try_into()?;

        self.context.rename_path_mark(&from, &to);

        self.reveal_path_in_explorer(&to)?;

        self.lsp_manager().send_message(
            from.clone(),
            FromEditor::WorkspaceDidRenameFiles {
                old: from.clone(),
                new: to,
            },
        )?;
        self.layout.remove_suggestive_editor(&from);
        Ok(())
    }

    fn copy_file(&mut self, from: CanonicalizedPath, to: PathBuf) -> anyhow::Result<()> {
        use std::fs;
        self.add_path_parent(&to)?;
        fs::copy(from.clone(), to.clone())?;
        self.layout.refresh_file_explorer(&self.context)?;
        let to = to.try_into()?;
        self.reveal_path_in_explorer(&to)?;
        self.lsp_manager().send_message(
            from.clone(),
            FromEditor::WorkspaceDidCreateFiles { file_path: to },
        )?;
        self.layout.remove_suggestive_editor(&from);
        Ok(())
    }

    fn add_path_parent(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(new_dir) = path.parent() {
            std::fs::create_dir_all(new_dir)?;
        }
        Ok(())
    }

    fn add_path(&mut self, path: String) -> anyhow::Result<()> {
        if PathBuf::from(path.clone()).exists() {
            return Err(anyhow::anyhow!("The path \"{}\" already exists", path));
        };
        if path.ends_with(&std::path::MAIN_SEPARATOR.to_string()) {
            std::fs::create_dir_all(path.clone())?;
        } else {
            let path: PathBuf = path.clone().into();
            self.add_path_parent(&path)?;
            std::fs::File::create(&path)?;
        }
        self.layout.refresh_file_explorer(&self.context)?;
        let path: CanonicalizedPath = path.try_into()?;
        self.reveal_path_in_explorer(&path)?;
        self.lsp_manager().send_message(
            path.clone(),
            FromEditor::WorkspaceDidCreateFiles { file_path: path },
        )?;
        Ok(())
    }

    fn request_syntax_highlight(
        &self,
        component_id: ComponentId,
        batch_id: SyntaxHighlightRequestBatchId,
        language: Language,
        content: String,
    ) -> anyhow::Result<()> {
        if let Some(sender) = &self.syntax_highlight_request_sender {
            sender.send(SyntaxHighlightRequest {
                component_id,
                batch_id,
                language,
                source_code: content,
            })?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn get_current_selected_texts(&self) -> Vec<String> {
        self.current_component()
            .borrow()
            .editor()
            .get_selected_texts()
    }

    #[cfg(test)]
    pub fn get_current_editor(&self) -> Rc<RefCell<dyn Component>> {
        let component = self
            .layout
            .get_component_by_kind(ComponentKind::SuggestiveEditor)
            .unwrap();
        component.clone()
    }

    #[cfg(test)]
    pub fn get_file_content(&self, path: &CanonicalizedPath) -> String {
        self.layout
            .get_existing_editor(path)
            .unwrap()
            .borrow()
            .content()
    }

    pub fn handle_dispatch_editor(
        &mut self,
        dispatch_editor: DispatchEditor,
    ) -> anyhow::Result<()> {
        self.handle_dispatch_editor_custom(dispatch_editor, self.current_component())
    }

    fn handle_dispatch_editor_custom(
        &mut self,
        dispatch_editor: DispatchEditor,
        component: Rc<RefCell<dyn Component>>,
    ) -> anyhow::Result<()> {
        // Call the component's handle_dispatch_editor method
        let dispatches = component
            .borrow_mut()
            .handle_dispatch_editor(&mut self.context, dispatch_editor.clone())?;

        self.handle_dispatches(dispatches)?;

        if self.is_running_as_embedded() {
            /*
            Note: we always send the latest selection set & selection mode to VS Code
              regardless of whether they actually changes after handling
              `dispatch_editor`. This is the simplest and most reliable way
              to ensure the updated selection set is sent to VS Code,
              rather than tracking all possible paths that lead to selection updates.
              We are sacrificing a little performance (by sending the same selection set to VS Code occasionally)
              in exchange for better code maintainability and behavioral correctness.
            */
            let other_dispatches = Dispatches::default()
                .append(component.borrow().editor().dispatch_selection_changed())
                .append(Dispatch::ToHostApp(ToHostApp::MarksChanged(
                    component.borrow().id(),
                    self.context.get_marks(component.borrow().editor().path()),
                )))
                .append(
                    component
                        .borrow()
                        .editor()
                        .dispatch_selection_mode_changed(),
                )
                .append(Dispatch::ToHostApp(ToHostApp::ModeChanged));
            self.handle_dispatches(other_dispatches)?;
        };

        Ok(())
    }

    fn get_repo_git_hunks(&mut self, diff_mode: git::DiffMode) -> anyhow::Result<()> {
        let working_directory = self.working_directory().clone();
        let repo = git::GitRepo::try_from(&working_directory)?;
        let diffs = repo.diffs(diff_mode)?;
        self.set_quickfix_list_type(
            ResponseContext::default().set_description("Git Hunks"),
            QuickfixListType::Items(
                diffs
                    .into_iter()
                    .flat_map(|file_diff| {
                        file_diff
                            .hunks()
                            .iter()
                            .filter_map(|hunk| {
                                let buffer = Buffer::from_path(file_diff.path(), false).ok()?;
                                let line_range = hunk.line_range();
                                let location = Location {
                                    path: file_diff.path().clone(),
                                    range: buffer
                                        .line_range_to_char_index_range(line_range)
                                        .ok()?,
                                };
                                Some(QuickfixListItem::new(location, hunk.to_info(), None))
                            })
                            .collect_vec()
                    })
                    .collect_vec(),
            ),
        )
    }

    #[cfg(test)]
    fn set_global_title(&mut self, title: String) {
        self.global_title = Some(title)
    }

    pub fn get_current_file_path(&self) -> Option<CanonicalizedPath> {
        self.current_component().borrow().path()
    }

    fn set_global_mode(&mut self, mode: Option<GlobalMode>) -> anyhow::Result<()> {
        self.context.set_mode(mode.clone());
        if let Some(GlobalMode::QuickfixListItem) = mode {
            self.goto_quickfix_list_item(Movement::Current(IfCurrentNotFound::LookForward))?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn context(&self) -> &Context {
        &self.context
    }

    fn update_local_search_config(
        &mut self,
        update: LocalSearchConfigUpdate,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        run_search_after_config_updated: bool,
        component_id: Option<ComponentId>,
    ) -> Result<(), anyhow::Error> {
        self.context.update_local_search_config(update, scope);
        if run_search_after_config_updated {
            match scope {
                Scope::Local => self.local_search(if_current_not_found, component_id)?,
                Scope::Global => {
                    self.global_search()?;
                }
            }
        }

        Ok(())
    }

    fn update_global_search_config(
        &mut self,
        update: GlobalSearchConfigUpdate,
    ) -> anyhow::Result<()> {
        self.context.update_global_search_config(update)?;
        self.global_search()?;
        Ok(())
    }

    fn words(&self) -> Vec<DropdownItem> {
        self.current_component()
            .borrow()
            .editor()
            .buffer()
            .words()
            .into_iter()
            .map(|word| {
                DropdownItem::new(word.clone()).set_dispatches(Dispatches::one(Dispatch::ToEditor(
                    ReplaceCurrentSelectionWith(word),
                )))
            })
            .collect_vec()
    }

    fn cycle_marked_file(&mut self, movement: Movement) -> anyhow::Result<()> {
        if let Some(next_file_path) = {
            let merged_file_paths_map: IndexMap<CanonicalizedPath, bool> = {
                let marked_file_paths = self.context.get_marked_files();
                let buffer_file_paths = self.layout.get_opened_files();

                // marked status is prioritized: overwrites the value of existing key
                marked_file_paths
                    .iter()
                    .map(|path| ((*path).clone(), true))
                    .chain(
                        buffer_file_paths
                            .iter()
                            .filter(|path| !marked_file_paths.contains(path))
                            .map(|path| (path.clone(), false)),
                    )
                    .collect()
            };

            self.get_current_file_path()
                .and_then(|current_file_path| {
                    if let Some(current_index) = merged_file_paths_map
                        .iter()
                        .position(|(path, _)| path == &current_file_path)
                    {
                        let next_index = self.calculate_next_index(
                            &merged_file_paths_map,
                            current_index,
                            movement,
                        );
                        // We are doing defensive programming here
                        // to ensure that Ki editor never crashes
                        return merged_file_paths_map
                            .get_index(next_index)
                            .map(|(path, _)| path);
                    }
                    None
                })
                .or_else(|| merged_file_paths_map.first().map(|(path, _)| path))
                .cloned()
        } {
            let next_file_path = next_file_path.clone();
            if let Err(err) = self.open_file(&next_file_path.clone(), BufferOwner::User, true, true)
            {
                // If the file failed to open, show the error.
                // The failure reasons might be:
                // - the file no longer exists
                // - the file is not a file
                //
                // In such cases we should remove it from the list of marked files,
                // and then cycle to the next file.
                //
                // The removal is necessary otherwise this file will become an
                // an obstacle that prevents cycle_mark_file from passing through.
                self.context.toggle_path_mark(next_file_path.clone());
                self.show_global_info(Info::new(
                    "Cycle marked file error".to_string(),
                    format!(
                        "The file mark {:?} is removed from the list as it cannot be opened due to the following error:\n\n{err:?}",
                        next_file_path
                            .try_display_relative_to(self.context.current_working_directory())
                    ),
                ));
                self.cycle_marked_file(movement)?;
            }
        }
        Ok(())
    }

    fn calculate_next_index(
        &self,
        merged_file_paths_map: &IndexMap<CanonicalizedPath, bool>,
        current_index: usize,
        movement: Movement,
    ) -> usize {
        let len = merged_file_paths_map.len();
        if len < 1 {
            return current_index;
        }
        let marked_indices: NonEmpty<usize> = NonEmpty::from_vec(
            merged_file_paths_map
                .iter()
                .enumerate()
                .filter_map(|(index, (_, is_marked))| is_marked.then_some(index))
                .collect(),
        )
        .unwrap(); // len >= 1 as checked in previous conditional.

        match movement {
            Movement::First => *marked_indices.first(),
            Movement::Last => *marked_indices.last(),
            Movement::Left => *marked_indices
                .iter()
                .rev()
                .find(|index| **index < current_index)
                .unwrap_or(marked_indices.last()), // wrap to the other end
            Movement::Right => *marked_indices
                .iter()
                .find(|index| **index > current_index)
                .unwrap_or(marked_indices.first()), // wrap to the other end
            Movement::Previous => (0..merged_file_paths_map.len())
                .rev()
                .find(|index| *index < current_index)
                .unwrap_or(current_index),
            Movement::Next => (0..merged_file_paths_map.len())
                .find(|index| *index > current_index)
                .unwrap_or(current_index),
            _ => current_index,
        }
    }

    #[cfg(test)]
    pub fn get_current_component_content(&self) -> String {
        self.current_component().borrow().editor().content()
    }

    #[cfg(test)]
    pub fn get_buffer_contents_map(&self) -> BufferContentsMap {
        self.layout.get_buffer_contents_map()
    }

    #[cfg(test)]
    fn handle_key_events(&mut self, key_events: Vec<event::KeyEvent>) -> anyhow::Result<()> {
        for key_event in key_events.into_iter() {
            self.handle_event(Event::Key(key_event.to_owned()))?;
        }
        Ok(())
    }

    pub fn handle_dispatch_suggestive_editor(
        &mut self,
        dispatch: DispatchSuggestiveEditor,
    ) -> anyhow::Result<()> {
        if let Some(component) = self.layout.get_component_by_kind(ComponentKind::Prompt) {
            let dispatches = component
                .borrow_mut()
                .as_any_mut()
                .downcast_mut::<Prompt>()
                .ok_or_else(|| anyhow::anyhow!("App::handle_dispatch_suggestive_editor Failed to downcast current component to Prompt"))?
                .handle_dispatch_suggestive_editor(
                    dispatch,
                )?;
            self.handle_dispatches(dispatches)
        } else if let Some(component) = self
            .layout
            .get_component_by_kind(ComponentKind::SuggestiveEditor)
        {
            let dispatches = component
                .borrow_mut()
                .as_any_mut()
                .downcast_mut::<SuggestiveEditor>()
                .ok_or_else(|| anyhow::anyhow!("App::handle_dispatch_suggestive_editor Failed to downcast current component to SuggestiveEditor"))?
                .handle_dispatch(
                    dispatch,
                )?;
            self.handle_dispatches(dispatches)
        } else {
            // Ignore this dispatch if the current component is neither Prompt nor SuggestiveEditor
            // We don't raise an error here because in some cases, it is possible that the Prompt/SuggestiveEditor
            // has been removed before this dispatch can be handled.
            Ok(())
        }
    }

    #[cfg(test)]
    pub fn completion_dropdown_is_open(&self) -> bool {
        self.layout.completion_dropdown_is_open()
    }

    pub fn current_completion_dropdown(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.current_completion_dropdown()
    }

    #[cfg(test)]
    pub fn current_completion_dropdown_info(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout
            .get_component_by_kind(ComponentKind::DropdownInfo)
    }

    fn open_prompt(&mut self, prompt_config: PromptConfig) -> anyhow::Result<()> {
        if self.is_running_as_embedded() {
            self.open_prompt_embedded(prompt_config)
        } else {
            self.open_prompt_non_embedded(prompt_config)
        }
    }

    fn open_prompt_non_embedded(&mut self, prompt_config: PromptConfig) -> anyhow::Result<()> {
        // Initialize the incremental search matches
        // so that the possible selections highlights will be "cleared" (i.e., not rendered)
        self.current_component()
            .borrow_mut()
            .editor_mut()
            .initialize_incremental_search_matches();

        let (prompt, dispatches) = Prompt::new(prompt_config, &self.context);

        self.layout.add_and_focus_prompt(
            ComponentKind::Prompt,
            Rc::new(RefCell::new(prompt)),
            &self.context,
        );
        self.handle_dispatches(dispatches)
    }

    fn open_prompt_embedded(&mut self, prompt_config: PromptConfig) -> anyhow::Result<()> {
        let history = match &prompt_config.on_enter {
            PromptOnEnter::ParseCurrentLine {
                history_key: key, ..
            } => self.context.get_prompt_history(*key),
            _ => Default::default(),
        };

        let items = prompt_config
            .items()
            .iter()
            .map(|item| ki_protocol_types::PromptItem {
                label: item.display(),
                details: item.info().map(|info| info.content()).cloned(),
            })
            .chain(
                history
                    .into_iter()
                    .map(|label| ki_protocol_types::PromptItem {
                        label,
                        details: None,
                    }),
            )
            .collect();

        let title = prompt_config.title.clone();

        self.last_prompt_config = Some(prompt_config);

        self.integration_event_sender
            .emit_event(crate::integration_event::IntegrationEvent::PromptOpened { title, items });
        Ok(())
    }

    fn prompt_entered(&mut self, entry: String) -> anyhow::Result<()> {
        let Some(prompt_config) = self.last_prompt_config.take() else {
            return Ok(());
        };
        let dispatches = match prompt_config.on_enter {
            PromptOnEnter::ParseCurrentLine {
                parser: dispatch_parser,
                history_key: prompt_history_key,
                ..
            } => dispatch_parser
                .parse(&entry)?
                .append(Dispatch::PushPromptHistory {
                    key: prompt_history_key,
                    line: entry,
                }),
            _ => Default::default(),
        };
        self.handle_dispatches(dispatches)
    }
    fn render_dropdown(
        &mut self,
        editor: Rc<RefCell<Editor>>,
        render: DropdownRender,
    ) -> Result<(), anyhow::Error> {
        let dispatches = editor
            .borrow_mut()
            .render_dropdown(&mut self.context, &render)?;
        editor.borrow_mut().set_title(render.title);

        match render.info {
            Some(info) => {
                self.layout.show_dropdown_info(info, &self.context)?;
            }
            _ => self.layout.hide_dropdown_info(),
        }
        self.handle_dispatches(dispatches)
    }

    #[cfg(test)]
    pub fn get_dropdown_infos_count(&self) -> usize {
        self.layout.get_dropdown_infos_count()
    }

    pub fn render_quickfix_list(&mut self) -> anyhow::Result<()> {
        if self.context.quickfix_list_items_count() > 10000 {
            return Ok(());
        };
        let (editor, dispatches) = self
            .layout
            .show_quickfix_list(self.context.quickfix_list(), &self.context)?;

        let editor = editor.borrow();
        let buffer = editor.buffer();
        if let Some(language) = buffer.language() {
            self.request_syntax_highlight(
                editor.id(),
                buffer.batch_id().clone(),
                language,
                buffer.content(),
            )?;
        };
        self.handle_dispatches(dispatches)
    }

    fn show_editor_info(&mut self, info: Info) -> anyhow::Result<()> {
        if self.is_running_as_embedded() {
            self.integration_event_sender
                .emit_event(IntegrationEvent::ShowInfo {
                    info: Some(info.display()),
                });
        }
        self.layout.show_editor_info(info, &self.context)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn editor_info_contents(&self) -> Vec<String> {
        self.layout.editor_info_contents()
    }

    #[cfg(test)]
    pub fn global_info_contents(&self) -> Vec<String> {
        self.layout.global_info_contents()
    }

    fn reveal_path_in_explorer(&mut self, path: &CanonicalizedPath) -> anyhow::Result<()> {
        let dispatches = self.layout.reveal_path_in_explorer(path, &self.context)?;
        self.send_file_watcher_input(FileWatcherInput::SyncFileExplorerExpandedFolders(
            self.layout
                .file_explorer_expanded_folders()
                .into_iter()
                // Need to include the current working directory (cwd)
                // otherwise path modifications of files that are parked directly under the cwd
                // will not refresh the file explorer.
                .chain(Some(self.context.current_working_directory().clone()))
                .collect(),
        ));
        self.handle_dispatches(dispatches)
    }

    #[cfg(test)]
    pub fn file_explorer_content(&self) -> String {
        self.layout.file_explorer_content()
    }

    fn open_code_actions_picker(
        &mut self,
        code_actions: Vec<crate::lsp::code_action::CodeAction>,
    ) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig::new(
            "Code Actions".to_string(),
            PromptOnEnter::SelectsFirstMatchingItem {
                items: PromptItems::Precomputed(
                    code_actions
                        .into_iter()
                        .map(move |code_action| code_action.into())
                        .collect(),
                ),
            },
        ))?;
        Ok(())
    }

    fn close_current_window_and_focus_parent(&mut self) {
        self.layout.close_current_window_and_focus_parent();
        self.integration_event_sender
            .emit_event(IntegrationEvent::ShowInfo { info: None });
    }

    pub fn opened_files_count(&self) -> usize {
        self.layout.get_opened_files().len()
    }

    #[cfg(test)]
    pub fn global_info(&self) -> Option<String> {
        self.layout.global_info()
    }

    #[cfg(test)]
    pub fn get_component_by_kind(&self, kind: ComponentKind) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.get_component_by_kind(kind)
    }

    fn hide_editor_info(&mut self) {
        self.layout.hide_editor_info()
    }

    #[cfg(test)]
    pub fn components_order(&self) -> Vec<ComponentKind> {
        self.layout
            .components()
            .into_iter()
            .map(|c| c.kind())
            .collect_vec()
    }

    fn handle_signature_help(
        &mut self,
        signature_help: Option<crate::lsp::signature_help::SignatureHelp>,
    ) -> anyhow::Result<()> {
        if let Some(info) = signature_help.and_then(|s| s.into_info()) {
            if self.current_component().borrow().editor().mode
                == crate::components::editor::Mode::Insert
            {
                self.show_editor_info(info)?;
            }
        } else {
            self.hide_editor_info()
        }
        Ok(())
    }

    fn push_history_prompt(&mut self, key: PromptHistoryKey, line: String) {
        self.context.push_history_prompt(key, line)
    }

    fn open_theme_picker(&mut self) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig::new(
                "Theme".to_string(),
                PromptOnEnter::SelectsFirstMatchingItem {
                    items: PromptItems::Precomputed(
                        crate::themes::theme_descriptor::all()
                            .into_iter()
                            .enumerate()
                            .map(|(index, theme_descriptor)| {
                                DropdownItem::new(theme_descriptor.name().to_string())
                                    .set_rank(Some(Box::from([index].to_vec())))
                                    .set_on_focused(Dispatches::one(
                                        Dispatch::SetThemeFromDescriptor(theme_descriptor.clone()),
                                    ))
                                    .set_dispatches(Dispatches::one(
                                        Dispatch::SetThemeFromDescriptor(theme_descriptor),
                                    ))
                            })
                            .collect_vec(),
                    ),
                },
            )
            .set_on_cancelled(Some(Dispatches::one(Dispatch::SetTheme(
                self.context.theme().clone(),
            )))),
        )
    }

    fn open_keyboard_layout_picker(&mut self) -> anyhow::Result<()> {
        let embedded = self.context.is_running_as_embedded();
        self.open_prompt(PromptConfig::new(
            "Keyboard Layout".to_string(),
            if embedded {
                PromptOnEnter::ParseCurrentLine {
                    parser: DispatchParser::SetKeyboardLayoutKind,
                    history_key: PromptHistoryKey::KeyboardLayout,
                    current_line: None,
                    suggested_items: Default::default(),
                }
            } else {
                PromptOnEnter::SelectsFirstMatchingItem {
                    items: PromptItems::Precomputed(
                        KeyboardLayoutKind::iter()
                            .map(|keyboard_layout| {
                                DropdownItem::new(keyboard_layout.display().to_string())
                                    .set_dispatches(if embedded {
                                        Dispatches::default()
                                    } else {
                                        Dispatches::one(Dispatch::SetKeyboardLayoutKind(
                                            keyboard_layout,
                                        ))
                                    })
                            })
                            .collect_vec(),
                    ),
                }
            },
        ))
    }

    fn update_current_completion_item(
        &mut self,
        completion_item: CompletionItem,
    ) -> anyhow::Result<()> {
        self.handle_dispatch_suggestive_editor(
            DispatchSuggestiveEditor::UpdateCurrentCompletionItem(Box::new(completion_item)),
        )
    }

    #[cfg(test)]
    pub fn lsp_request_sent(&mut self, from_editor: &FromEditor) -> bool {
        self.lsp_manager().lsp_request_sent(from_editor)
    }

    fn open_pipe_to_shell_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig::new(
            "Pipe to shell".to_string(),
            PromptOnEnter::ParseCurrentLine {
                parser: DispatchParser::PipeToShell,
                history_key: PromptHistoryKey::PipeToShell,
                current_line: None,
                suggested_items: Default::default(),
            },
        ))
    }

    fn use_last_non_contiguous_selection_mode(
        &mut self,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<()> {
        if let Some(selection_mode) = self.context.last_non_contiguous_selection_mode() {
            match selection_mode {
                Either::Left(selection_mode) => self.handle_dispatch_editor(
                    DispatchEditor::SetSelectionMode(if_current_not_found, selection_mode.clone()),
                )?,
                Either::Right(global_mode) => self.handle_dispatches(Dispatches::new(
                    [
                        Dispatch::SetGlobalMode(Some(global_mode.clone())),
                        Dispatch::ToEditor(MoveSelection(match if_current_not_found {
                            IfCurrentNotFound::LookForward => Movement::Right,
                            IfCurrentNotFound::LookBackward => Movement::Left,
                        })),
                    ]
                    .to_vec(),
                ))?,
            }
        }
        Ok(())
    }

    fn open_filter_selections_prompt(&mut self, maintain: bool) -> anyhow::Result<()> {
        let config = self.context.local_search_config(Scope::Local);
        let mode = config.mode;
        self.open_prompt(PromptConfig::new(
            format!(
                "{} selections matching search ({})",
                if maintain { "Maintain" } else { "Remove" },
                mode.display()
            ),
            PromptOnEnter::ParseCurrentLine {
                parser: DispatchParser::FilterSelectionMatchingSearch { maintain },
                history_key: PromptHistoryKey::FilterSelectionsMatchingSearch { maintain },
                current_line: None,
                suggested_items: Default::default(),
            },
        ))
    }

    fn navigate_back(&mut self) -> anyhow::Result<()> {
        while let Some(location) = self.context.location_previous() {
            if location.path.exists() {
                self.push_current_location_into_navigation_history(false);
                self.go_to_location(&location, false)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn navigate_forward(&mut self) -> anyhow::Result<()> {
        while let Some(location) = self.context.location_next() {
            if location.path.exists() {
                self.push_current_location_into_navigation_history(true);
                self.go_to_location(&location, false)?
            }
        }
        Ok(())
    }

    fn push_current_location_into_navigation_history(&mut self, backward: bool) {
        // TODO: should include scroll offset as well
        // so that when the user navigates back, it really feels the same
        if let Some(path) = self.current_component().borrow().editor().path() {
            let range = self
                .current_component()
                .borrow()
                .editor()
                .current_selection_range();
            let location = Location { path, range };
            self.context.push_location_history(location, backward)
        }
    }

    fn mark_file_and_toggle_mark(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.get_current_file_path() {
            let _ = self.context.mark_file(path);
        }
        let dispatches = self
            .current_component()
            .borrow_mut()
            .editor_mut()
            .toggle_marks();
        let _ = self.handle_dispatches(dispatches);
        Ok(())
    }

    fn toggle_file_mark(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.get_current_file_path() {
            if let Some(new_path) = self.context.toggle_path_mark(path).cloned() {
                self.open_file(&new_path, BufferOwner::User, true, true)?;
            }
        }
        Ok(())
    }

    fn mode_changed(&self) {
        // This dispatch is handled by the VSCode integration to send mode change notifications
        // No action needed here as the mode has already been changed in the editor

        // Get the current component and its mode
        let component = self.current_component();
        let component_ref = component.borrow();
        let editor = component_ref.editor();
        // Emit an integration event for the mode change
        self.integration_event_sender.emit_event(
            crate::integration_event::IntegrationEvent::ModeChanged {
                component_id: editor.id(),
                mode: editor.mode.clone(),
            },
        );
    }

    fn selection_changed(
        &self,
        component_id: ComponentId,
        selections: Vec<crate::selection::Selection>,
    ) {
        // Convert component_id to usize for integration event
        self.integration_event_sender.emit_event(
            crate::integration_event::IntegrationEvent::SelectionChanged {
                component_id,
                selections: selections.clone(),
            },
        );
    }

    fn jumps_changed(&self, component_id: ComponentId, jumps: Vec<(char, CharIndex)>) {
        self.integration_event_sender.emit_event(
            crate::integration_event::IntegrationEvent::JumpsChanged {
                component_id,
                jumps,
            },
        );
    }

    fn marks_updated(
        &self,
        component_id: ComponentId,
        marks: Vec<crate::char_index_range::CharIndexRange>,
    ) {
        self.integration_event_sender.emit_event(
            crate::integration_event::IntegrationEvent::MarksChanged {
                component_id,
                marks,
            },
        );
    }

    fn selection_mode_changed(&self, selection_mode: SelectionMode) {
        // This dispatch is handled by the VSCode integration to send mode change notifications
        // No action needed here as the mode has already been changed in the editor

        // Get the current component and its mode
        let component = self.current_component();
        let component_ref = component.borrow();
        let editor = component_ref.editor();
        // Emit an integration event for the mode change
        self.integration_event_sender.emit_event(
            crate::integration_event::IntegrationEvent::SelectionModeChanged {
                component_id: editor.id(),
                selection_mode,
            },
        );
    }

    fn keyboard_layout_changed(&self) {
        self.integration_event_sender.emit_event(
            crate::integration_event::IntegrationEvent::KeyboardLayoutChanged(
                self.context.keyboard_layout_kind().display(),
            ),
        );
    }

    fn is_running_as_embedded(&self) -> bool {
        self.context.is_running_as_embedded()
    }

    pub fn take_queued_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.queued_events)
    }

    fn handle_targeted_event(
        &mut self,
        event: Event,
        path: Option<CanonicalizedPath>,
        content_hash: u32,
    ) -> anyhow::Result<()> {
        // If the current component kind is a not a SuggestiveEditor, we handle the event directly
        if self.layout.get_current_component_kind() != Some(ComponentKind::SuggestiveEditor) {
            self.handle_event(event)?;
            Ok(())
        } else if let Some(path) = path {
            let component = self.open_file(&path, BufferOwner::User, false, true)?;

            // Compare the checksum of of the content of the buffer in Ki with that of the host application (e.g. VS Code)
            // This step is necessary to detect unsynchronized buffer
            if content_hash != crc32fast::hash(component.borrow().content().as_bytes()) {
                // If the buffer is desync, request the latest content
                // before handling this event
                self.integration_event_sender
                    .emit_event(IntegrationEvent::SyncBufferRequest { path });

                // Suspend this event until the buffer content is synced
                self.queued_events.push(event);

                return Ok(());
            }

            let dispatches = component
                .borrow_mut()
                .handle_event(&self.context, event.clone())?;
            self.handle_dispatches(dispatches)
        } else {
            // If no path is provided, handle the event for the current component
            self.handle_event(event)?;
            Ok(())
        }
    }

    fn handle_to_host_app(&mut self, to_host_app: ToHostApp) -> anyhow::Result<()> {
        match to_host_app {
            ToHostApp::BufferEditTransaction { path, edits } => {
                // Emit an integration event for the buffer change
                self.integration_event_sender.emit_event(
                    crate::integration_event::IntegrationEvent::BufferChanged {
                        path: path.clone(),
                        edits: edits.clone(),
                    },
                );
            }
            ToHostApp::ModeChanged => self.mode_changed(),
            ToHostApp::SelectionModeChanged(selection_mode) => {
                self.selection_mode_changed(selection_mode)
            }
            ToHostApp::SelectionChanged {
                component_id,
                selections,
            } => self.selection_changed(component_id, selections),
            ToHostApp::JumpsChanged {
                component_id,
                jumps,
            } => self.jumps_changed(component_id, jumps),
            ToHostApp::PromptEntered(entry) => self.prompt_entered(entry)?,
            ToHostApp::MarksChanged(component_id, marks) => self.marks_updated(component_id, marks),
        }
        Ok(())
    }

    fn handle_from_host_app(&mut self, from_host_app: FromHostApp) -> anyhow::Result<()> {
        match from_host_app {
            FromHostApp::TargetedEvent {
                event,
                path,
                content_hash,
            } => self.handle_targeted_event(event, path, content_hash)?,
        }
        Ok(())
    }

    fn open_search_prompt_with_prior_change(
        &mut self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        prior_change: Option<PriorChange>,
    ) -> anyhow::Result<()> {
        self.current_component()
            .borrow_mut()
            .editor_mut()
            .handle_prior_change(prior_change);
        self.open_search_prompt(scope, if_current_not_found, None)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn lsp_server_initialized_args(&mut self) -> Option<(LanguageId, Vec<CanonicalizedPath>)> {
        self.lsp_manager().lsp_server_initialized_args()
    }

    fn handle_nucleo_debounced(&mut self) -> Result<(), anyhow::Error> {
        let dispatches = {
            let component = self.layout.get_current_component();
            let mut component_mut = component.borrow_mut();
            let Some(prompt) = component_mut.as_any_mut().downcast_mut::<Prompt>() else {
                return Ok(());
            };

            let viewport_height = self
                .current_completion_dropdown()
                .map(|component| component.borrow().rectangle().height)
                .unwrap_or(10);

            prompt.handle_nucleo_updated(viewport_height)
        };
        self.handle_dispatches(dispatches)
    }

    fn handle_dropdown_filter_updated(&mut self, filter: String) -> anyhow::Result<()> {
        {
            let component = self.layout.get_current_component();
            let mut component_mut = component.borrow_mut();
            let Some(prompt) = component_mut.as_any_mut().downcast_mut::<Prompt>() else {
                return Ok(());
            };
            prompt.reparse_pattern(&filter);
        }
        self.handle_nucleo_debounced()
    }

    #[cfg(test)]
    fn set_system_clipboard_html(&self, html: &str, alt_text: &str) -> anyhow::Result<()> {
        Ok(arboard::Clipboard::new()?.set_html(html, Some(alt_text))?)
    }

    fn restore_session(&mut self) {
        // This condition is necessary, because user might have opened a file by passing
        // a path argument to the Ki CLI
        if self.opened_files_count() == 0 {
            // Try to go to a marked file, if there are loaded marked file from the persistence
            let _ = self.cycle_marked_file(Movement::Right);
        }
    }

    fn add_quickfix_list_entries(&mut self, matches: Vec<Match>) -> anyhow::Result<()> {
        // Jump to quickfix item if the quickfix list was empty
        let go_to_quickfix_item = self.context.quickfix_list_items_count() == 0;
        let items = matches
            .into_iter()
            .map(|m| QuickfixListItem::new(m.location, None, Some(m.line)))
            .collect_vec();
        self.context.extend_quickfix_list_items(items);

        self.render_quickfix_list()?;
        if go_to_quickfix_item {
            self.goto_quickfix_list_item(Movement::Current(IfCurrentNotFound::LookForward))?;
        }
        Ok(())
    }

    fn handle_applied_edits(&mut self, path: CanonicalizedPath, edits: Vec<Edit>) {
        self.context.handle_applied_edits(path, edits)
    }

    #[cfg(test)]
    pub fn wait_for_app_message(
        &mut self,
        app_message_matcher: &lazy_regex::Lazy<regex::Regex>,
        timeout: Option<Duration>,
    ) -> anyhow::Result<()> {
        use std::time::Instant;

        let start_time = Instant::now();
        let timeout = timeout.unwrap_or_else(|| Duration::from_secs(5));
        while (Instant::now() - start_time) < timeout {
            if let Ok(app_message) = self.receiver.try_recv() {
                let string = format!("{app_message:?}");
                self.process_message(app_message)?;
                if app_message_matcher.is_match(&string) {
                    return Ok(());
                }
            }
        }
        Err(anyhow::anyhow!(
            "No app message matching {} is received after {:?}.",
            app_message_matcher.as_str(),
            timeout,
        ))
    }

    #[cfg(test)]
    pub fn expect_app_message_not_received(
        &mut self,
        regex: &&'static lazy_regex::Lazy<regex::Regex>,
        timeout: &Duration,
    ) -> anyhow::Result<()> {
        use std::time::Instant;

        let start_time = Instant::now();
        while &(Instant::now() - start_time) < timeout {
            if let Ok(app_message) = self.receiver.try_recv() {
                let string = format!("{app_message:?}");
                self.process_message(app_message)?;
                if regex.is_match(&string) {
                    return Err(anyhow::anyhow!(
                    "Expected no app message matching {} is received within {timeout:?}, but got {string:?}",
                    regex.as_str(),
                ));
                }
            }
        }
        Ok(())
    }

    fn get_diff(before: &str, after: &str) -> String {
        let input = imara_diff::InternedInput::new(before, after);
        let mut diff = imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input);
        diff.postprocess_lines(&input);

        diff.unified_diff(
            &imara_diff::BasicLineDiffPrinter(&input.interner),
            imara_diff::UnifiedDiffConfig::default(),
            &input,
        )
        .to_string()
    }

    fn show_buffer_save_conflict_prompt(
        &mut self,
        path: &CanonicalizedPath,
        content_editor: String,
        content_filesystem: String,
    ) -> anyhow::Result<()> {
        let items = PromptItems::Precomputed(
            [
                DropdownItem::new("Force Save".to_string())
                    .set_dispatches(Dispatches::one(Dispatch::ToEditor(
                        DispatchEditor::ForceSave,
                    )))
                    .set_info(Some(Info::new(
                        "Diff to be applied".to_string(),
                        Self::get_diff(&content_filesystem, &content_editor),
                    ))),
                DropdownItem::new("Force Reload".to_string())
                    .set_dispatches(Dispatches::one(Dispatch::ToEditor(
                        DispatchEditor::ReloadFile { force: true },
                    )))
                    .set_info(Some(Info::new(
                        "Diff to be applied".to_string(),
                        Self::get_diff(&content_editor, &content_filesystem),
                    ))),
                DropdownItem::new("Merge".to_string())
                    .set_dispatches(Dispatches::one(Dispatch::ToEditor(
                        DispatchEditor::MergeContent {
                            content_filesystem,
                            content_editor,
                            path: path.clone(),
                        },
                    )))
                    .set_info(Some(Info::new(
                        "Info".to_string(),
                        "Perform a 3-way merge where:

- ours     = content of file in the Editor
- theirs   = content of file in the Filesystem
- original = content of file in the latest Git commit

Conflict markers will be injected in areas that cannot be merged gracefully."
                            .to_string(),
                    ))),
            ]
            .into_iter()
            .collect(),
        );
        self.open_prompt(PromptConfig::new(
            format!(
                "Failed to save {}: The content of the file is newer.",
                path.try_display_relative_to(self.context.current_working_directory())
            ),
            PromptOnEnter::SelectsFirstMatchingItem { items },
        ))
    }

    fn handle_file_watcher_event(&mut self, event: FileWatcherEvent) -> anyhow::Result<()> {
        log::info!("Received file watcher event: {event:?}");
        match event {
            FileWatcherEvent::ContentModified(path) => {
                if path.is_file()
                    && self
                        .layout
                        .get_opened_files()
                        .iter()
                        .any(|opened_file| &path == opened_file)
                {
                    let component = self.open_file(&path, BufferOwner::User, false, false)?;
                    self.handle_dispatch_editor_custom(
                        DispatchEditor::ReloadFile { force: false },
                        component,
                    )?;
                }
            }
            FileWatcherEvent::PathCreated | FileWatcherEvent::PathRemoved(_) => {
                self.layout.refresh_file_explorer(&self.context)?;
            }
            FileWatcherEvent::PathRenamed {
                source,
                destination,
            } => {
                self.context
                    .handle_file_renamed(source.clone(), destination.clone());
                self.layout.refresh_file_explorer(&self.context)?;
                self.handle_dispatch_editor(DispatchEditor::PathRenamed {
                    source,
                    destination,
                })?
            }
        }
        Ok(())
    }

    fn send_file_watcher_input(&self, input: FileWatcherInput) {
        if let Some(sender) = self.file_watcher_input_sender.as_ref() {
            if let Err(error) = sender.send(input) {
                log::error!("[App::send_file_watcher_input] error = {error:?}")
            }
        }
    }

    fn open_workspace_symbols_picker(&mut self) -> anyhow::Result<()> {
        if self.is_running_as_embedded() {
            self.send_integration_event(IntegrationEvent::RequestLspWorkspaceSymbols);
            return Ok(());
        }

        let Some(path) = self.current_component().borrow().path() else {
            return Ok(());
        };
        self.open_prompt(
            PromptConfig::new(
                "Workspace Symbol".to_string(),
                PromptOnEnter::SelectsFirstMatchingItem {
                    items: PromptItems::BackgroundTask {
                        task: PromptItemsBackgroundTask::HandledByMainEventLoop,
                        on_nucleo_tick_debounced: {
                            let sender = self.sender.clone();
                            Callback::new(Arc::new(move |_| {
                                let _ = sender.send(AppMessage::NucleoTickDebounced);
                            }))
                        },
                    },
                },
            )
            .set_on_change(Some(PromptOnChangeDispatch::RequestWorkspaceSymbol(path))),
        )
    }

    fn handle_workspace_symbols(&self, symbols: Symbols) -> Result<(), anyhow::Error> {
        {
            let component = self.layout.get_current_component();
            let mut component_mut = component.borrow_mut();
            let Some(prompt) = component_mut.as_any_mut().downcast_mut::<Prompt>() else {
                return Ok(());
            };
            prompt.clear_and_update_matcher_items(
                symbols
                    .symbols
                    .clone()
                    .into_iter()
                    .map(|symbol| symbol.into())
                    .collect_vec(),
            );
        }
        Ok(())
    }

    fn get_component_by_id(&self, component_id: ComponentId) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.get_component_by_id(component_id)
    }

    fn get_and_handle_prompt_on_change_dispatches(&mut self) -> anyhow::Result<()> {
        let dispatches = {
            let component = self.layout.get_current_component();
            let mut component_mut = component.borrow_mut();
            let Some(prompt) = component_mut.as_any_mut().downcast_mut::<Prompt>() else {
                return Ok(());
            };
            prompt.get_on_change_dispatches()
        };
        self.handle_dispatches(dispatches)
    }

    fn set_incremental_search_config(
        &self,
        config: crate::context::LocalSearchConfig,
        component_id: Option<ComponentId>,
    ) {
        let Some(component_id) = component_id else {
            return;
        };
        let Some(component) = self.get_component_by_id(component_id) else {
            return;
        };
        let mut borrow = component.borrow_mut();
        borrow.editor_mut().set_incremental_search_config(config)
    }

    fn update_current_component_title(&self, title: String) {
        {
            let comp = self.current_component();
            let mut borrow = comp.borrow_mut();
            borrow.set_title(title)
        }
    }

    fn handle_script_dispatches(
        &mut self,
        script_dispatches: Vec<ScriptDispatch>,
    ) -> anyhow::Result<()> {
        self.handle_dispatches(Dispatches::new(
            script_dispatches
                .into_iter()
                .map(ScriptDispatch::into_app_dispatch)
                .collect_vec(),
        ))
    }

    fn execute_leader_key(&mut self, key: String) -> anyhow::Result<()> {
        if let Some((_, _, script)) = custom_keymap().into_iter().find(|(k, _, _)| k == &key) {
            let output = {
                let component = self.current_component();
                let borrow = component.borrow();
                let editor = borrow.editor();
                let context = ScriptInput {
                    current_file_path: self
                        .get_current_file_path()
                        .map(|path| path.display_absolute()),
                    selections: editor
                        .selection_set
                        .map(|selection| -> anyhow::Result<_> {
                            let range = editor
                                .buffer()
                                .char_index_range_to_position_range(selection.extended_range())?;
                            let content = editor
                                .buffer()
                                .slice(&selection.extended_range())?
                                .to_string();
                            Ok(crate::scripting::Selection { range, content })
                        })
                        .into_iter()
                        .try_collect()?,
                };

                script.execute(context)?

                // We need to drop `borrow` here, so that we can prevent double borrow
                // when `DispatchEditor`s are being handled
            };
            self.handle_script_dispatches(output.dispatches)?
        }
        Ok(())
    }

    fn open_and_mark_files(&mut self, paths: NonEmpty<CanonicalizedPath>) -> anyhow::Result<()> {
        self.open_file(paths.first(), BufferOwner::User, true, true)?;
        self.context.mark_files(paths);
        Ok(())
    }

    fn toggle_or_open_paths(&mut self) -> anyhow::Result<()> {
        let dispatches = {
            self.current_component()
                .borrow_mut()
                .as_any_mut()
                .downcast_mut::<FileExplorer>()
                .ok_or_else(|| {
                    anyhow::anyhow!("Unable to downcast current component to `FileExplorer`")
                })?
                .toggle_or_open_paths(&self.context)?
        };
        self.handle_dispatches(dispatches)
    }

    fn change_working_directory(&mut self, path: CanonicalizedPath) -> anyhow::Result<()> {
        self.context.change_working_directory(path)?;
        self.layout.refresh_file_explorer(&self.context)?;

        Ok(())
    }

    fn open_change_working_directory_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig::new(
                "Change Working Directory".to_string(),
                PromptOnEnter::ParseCurrentLine {
                    parser: DispatchParser::ChangeWorkingDirectory,
                    history_key: PromptHistoryKey::ChangeWorkingDirectory,
                    current_line: Some(format!(
                        "{}{}",
                        self.context.current_working_directory().display_absolute(),
                        std::path::MAIN_SEPARATOR
                    )),
                    suggested_items: Vec::new(),
                },
            )
            .set_on_change(Some(
                PromptOnChangeDispatch::UpdateSuggestedItemsWithChildPaths,
            )),
        )
    }

    fn working_directory(&self) -> &CanonicalizedPath {
        self.context.current_working_directory()
    }

    fn lsp_manager(&mut self) -> &mut LspManager {
        let working_directory = self.working_directory().clone();
        self.lsp_manager
            .entry(working_directory.to_path_buf().clone())
            .or_insert_with(|| LspManager::new(self.sender.clone(), working_directory))
    }

    #[cfg(test)]
    pub(crate) fn quickfix_list(&self) -> &QuickfixList {
        self.context.quickfix_list()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dimension {
    pub height: usize,
    pub width: usize,
}

impl Dimension {
    #[cfg(test)]
    pub fn area(&self) -> usize {
        self.height * self.width
    }

    #[cfg(test)]
    pub fn positions(&self) -> std::collections::HashSet<Position> {
        (0..self.height)
            .flat_map(|line| (0..self.width).map(move |column| Position { column, line }))
            .collect()
    }

    fn decrement_height(&self, global_title_bar_height: usize) -> Dimension {
        Dimension {
            height: self.height.saturating_sub(global_title_bar_height),
            width: self.width,
        }
    }
}

#[must_use]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Dispatches(Vec<Dispatch>);
impl From<Vec<Dispatch>> for Dispatches {
    fn from(value: Vec<Dispatch>) -> Self {
        Self(value)
    }
}
impl Dispatches {
    pub fn into_vec(self) -> Vec<Dispatch> {
        self.0
    }

    pub fn new(dispatches: Vec<Dispatch>) -> Dispatches {
        Dispatches(dispatches)
    }

    pub fn chain(self, other: Dispatches) -> Dispatches {
        self.0.into_iter().chain(other.0).collect_vec().into()
    }

    pub fn append(self, other: Dispatch) -> Dispatches {
        self.0.into_iter().chain(Some(other)).collect_vec().into()
    }

    pub fn append_some(self, dispatch: Option<Dispatch>) -> Dispatches {
        if let Some(dispatch) = dispatch {
            self.append(dispatch)
        } else {
            self
        }
    }

    pub fn one(edit: Dispatch) -> Dispatches {
        Dispatches(vec![edit])
    }

    pub fn empty() -> Dispatches {
        Dispatches(Default::default())
    }
}

#[must_use]
#[derive(Clone, Debug, PartialEq, NamedVariant)]
/// Dispatch are for child component to request action from the root node
pub enum Dispatch {
    SetTheme(crate::themes::Theme),
    SetThemeFromDescriptor(crate::themes::theme_descriptor::ThemeDescriptor),
    CloseCurrentWindow,
    OpenFilePicker(FilePickerKind),
    OpenSearchPrompt {
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    },
    OpenSearchPromptWithPriorChange {
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        prior_change: Option<PriorChange>,
    },
    OpenFile {
        path: CanonicalizedPath,
        owner: BufferOwner,
        focus: bool,
    },
    OpenFileFromPathBuf {
        path: PathBuf,
        owner: BufferOwner,
        focus: bool,
    },
    RequestCompletion,
    RequestSignatureHelp,
    RequestHover,
    RequestDefinitions(Scope),
    RequestDeclarations(Scope),
    RequestImplementations(Scope),
    RequestTypeDefinitions(Scope),
    RequestReferences {
        scope: Scope,
        include_declaration: bool,
    },
    PrepareRename,
    RequestCodeAction {
        diagnostics: Vec<lsp_types::Diagnostic>,
    },
    RenameSymbol {
        new_name: String,
    },
    DocumentDidChange {
        component_id: ComponentId,
        batch_id: SyntaxHighlightRequestBatchId,
        path: Option<CanonicalizedPath>,
        content: String,
        language: Option<Language>,
    },
    DocumentDidSave {
        path: CanonicalizedPath,
    },
    SetQuickfixList(QuickfixListType),
    GotoQuickfixListItem(Movement),
    ApplyWorkspaceEdit(WorkspaceEdit),
    ShowKeymapLegend(KeymapLegendConfig),
    /// This means showing a momentary layer.
    ShowKeymapLegendWithReleaseKey(KeymapLegendConfig, ReleaseKey),
    RemainOnlyCurrentComponent,

    #[cfg(test)]
    /// Used for testing
    Custom(String),
    ToEditor(DispatchEditor),
    RequestDocumentSymbols,
    GotoLocation(Location),
    OpenMoveToIndexPrompt(Option<PriorChange>),
    QuitAll,
    RevealInExplorer(CanonicalizedPath),
    OpenMovePathsPrompt,
    OpenDuplicateFilePrompt,
    OpenAddPathPrompt,
    DeletePaths(NonEmpty<CanonicalizedPath>),
    Null,
    MovePaths {
        sources: NonEmpty<CanonicalizedPath>,
        destinations: NonEmpty<PathBuf>,
    },
    CopyFile {
        from: CanonicalizedPath,
        to: PathBuf,
    },
    AddPath(String),
    RefreshFileExplorer,
    SetClipboardContent {
        copied_texts: CopiedTexts,
    },
    SetGlobalMode(Option<GlobalMode>),
    #[cfg(test)]
    HandleEvent(Event),
    #[cfg(test)]
    HandleKeyEvent(event::KeyEvent),
    #[cfg(test)]
    HandleKeyEvents(Vec<event::KeyEvent>),
    GetRepoGitHunks(git::DiffMode),
    SaveAll,
    #[cfg(test)]
    TerminalDimensionChanged(Dimension),
    #[cfg(test)]
    SetGlobalTitle(String),
    LspExecuteCommand {
        command: crate::lsp::code_action::Command,
    },
    UpdateLocalSearchConfig {
        update: LocalSearchConfigUpdate,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        run_search_after_config_updated: bool,
        /// If None, then this search will run in the current component
        component_id: Option<ComponentId>,
    },
    UpdateGlobalSearchConfig {
        update: GlobalSearchConfigUpdate,
    },
    Replace {
        scope: Scope,
    },
    #[cfg(test)]
    HandleLspNotification(LspNotification),
    CloseDropdown,
    RenderDropdown {
        render: DropdownRender,
    },
    #[cfg(test)]
    OpenPrompt {
        config: PromptConfig,
    },
    ShowEditorInfo(Info),
    ReceiveCodeActions(Vec<crate::lsp::code_action::CodeAction>),
    OtherWindow,
    CloseCurrentWindowAndFocusParent,
    CloseEditorInfo,
    CloseGlobalInfo,
    CycleMarkedFile(Movement),
    PushPromptHistory {
        key: PromptHistoryKey,
        line: String,
    },
    OpenThemePrompt,
    ResolveCompletionItem(lsp_types::CompletionItem),
    OpenPipeToShellPrompt,
    SetLastNonContiguousSelectionMode(Either<SelectionMode, GlobalMode>),
    UseLastNonContiguousSelectionMode(IfCurrentNotFound),
    SetLastActionDescription {
        long_description: String,
        short_description: Option<String>,
    },
    OpenFilterSelectionsPrompt {
        maintain: bool,
    },
    MoveToCompletionItem(Direction),
    OpenDeletePathsPrompt,
    SelectCompletionItem,
    SetKeyboardLayoutKind(KeyboardLayoutKind),
    OpenKeyboardLayoutPrompt,
    NavigateForward,
    NavigateBack,
    MarkFileAndToggleMark,
    ToggleFileMark,
    Suspend,

    ToHostApp(ToHostApp),
    FromHostApp(FromHostApp),
    OpenSurroundXmlPrompt,
    OpenSearchPromptWithCurrentSelection {
        scope: Scope,
        prior_change: Option<PriorChange>,
    },
    ShowGlobalInfo(Info),
    DropdownFilterUpdated(String),
    #[cfg(test)]
    SetSystemClipboardHtml {
        html: &'static str,
        alt_text: &'static str,
    },
    AddQuickfixListEntries(Vec<Match>),
    AppliedEdits {
        edits: Vec<Edit>,
        path: CanonicalizedPath,
    },
    ExecuteLeaderKey(String),
    ShowBufferSaveConflictPrompt {
        path: CanonicalizedPath,
        content_filesystem: String,
        content_editor: String,
    },
    RequestWorkspaceSymbols {
        query: String,
        path: CanonicalizedPath,
    },
    OpenWorkspaceSymbolsPrompt,
    GetAndHandlePromptOnChangeDispatches,
    SetIncrementalSearchConfig {
        config: crate::context::LocalSearchConfig,
        component_id: Option<ComponentId>,
    },
    UpdateCurrentComponentTitle(String),
    SaveMarks {
        path: CanonicalizedPath,
        marks: Vec<CharIndexRange>,
    },
    ToSuggestiveEditor(DispatchSuggestiveEditor),
    RequestCompletionDebounced,
    OpenAndMarkFiles(NonEmpty<CanonicalizedPath>),
    ToggleOrOpenPaths,
    ChangeWorkingDirectory(CanonicalizedPath),
    AddClipboardHistory(CopiedTexts),
    OpenChangeWorkingDirectoryPrompt,
}

/// Used to send notify host app about changes
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToHostApp {
    BufferEditTransaction {
        path: CanonicalizedPath,
        edits: Vec<ki_protocol_types::DiffEdit>,
    },
    ModeChanged,
    SelectionChanged {
        component_id: crate::components::component::ComponentId,
        selections: Vec<crate::selection::Selection>,
    },
    JumpsChanged {
        component_id: crate::components::component::ComponentId,
        jumps: Vec<(char, CharIndex)>,
    },
    SelectionModeChanged(SelectionMode),
    PromptEntered(String),
    MarksChanged(ComponentId, Vec<crate::char_index_range::CharIndexRange>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FromHostApp {
    TargetedEvent {
        event: Event,
        path: Option<CanonicalizedPath>,
        content_hash: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlobalSearchConfigUpdate {
    Config(GlobalSearchConfig),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalSearchConfigUpdate {
    #[cfg(test)]
    Mode(LocalSearchConfigMode),
    #[cfg(test)]
    Replacement(String),
    #[cfg(test)]
    Search(String),
    Config(crate::context::LocalSearchConfig),
}

#[derive(Clone, Debug, PartialEq)]
pub struct YesNoPrompt {
    pub title: String,
    pub yes: Box<Dispatch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilePickerKind {
    NonGitIgnored,
    GitStatus(git::DiffMode),
    Opened,
}
impl FilePickerKind {
    pub fn display(&self) -> String {
        match self {
            FilePickerKind::NonGitIgnored => "Not Git Ignored".to_string(),
            FilePickerKind::GitStatus(diff_mode) => format!("Git Status ({})", diff_mode.display()),
            FilePickerKind::Opened => "Opened".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestParams {
    pub path: CanonicalizedPath,
    pub position: Position,
    pub context: ResponseContext,
}
impl RequestParams {
    pub fn set_kind(self, scope: Option<Scope>) -> Self {
        Self {
            context: ResponseContext {
                scope,
                ..self.context
            },
            ..self
        }
    }

    pub fn set_description(self, description: &str) -> Self {
        Self {
            context: ResponseContext {
                description: Some(description.to_string()),
                ..self.context
            },
            ..self
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Copy)]
pub enum Scope {
    Local,
    Global,
}

#[derive(Debug)]
pub enum AppMessage {
    LspNotification(Box<LspNotification>),
    Event(Event),
    QuitAll,
    SyntaxHighlightResponse {
        component_id: ComponentId,
        batch_id: SyntaxHighlightRequestBatchId,
        highlighted_spans: HighlightedSpans,
    },
    // New variant for external dispatches
    NotifyError(std::io::Error),
    ExternalDispatch(Box<Dispatch>),
    NucleoTickDebounced,
    FileWatcherEvent(FileWatcherEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchParser {
    MoveSelectionByIndex,
    RenameSymbol,
    UpdateLocalSearchConfigSearch {
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        run_search_after_config_updated: bool,
    },
    AddPath,
    MovePaths {
        sources: NonEmpty<CanonicalizedPath>,
    },
    CopyFile {
        from: CanonicalizedPath,
    },
    #[cfg(test)]
    SetContent,
    PipeToShell,
    FilterSelectionMatchingSearch {
        maintain: bool,
    },
    SetKeyboardLayoutKind,
    SurroundXmlTag,
    #[cfg(test)]
    /// For testing only
    Null,
    ChangeWorkingDirectory,
}

impl DispatchParser {
    pub fn parse(&self, text: &str) -> anyhow::Result<Dispatches> {
        match self.clone() {
            DispatchParser::MoveSelectionByIndex => {
                let index = text.parse::<usize>()?.saturating_sub(1);
                Ok(Dispatches::new(
                    [Dispatch::ToEditor(MoveSelection(Movement::Index(index)))].to_vec(),
                ))
            }
            DispatchParser::RenameSymbol => Ok(Dispatches::new(vec![Dispatch::RenameSymbol {
                new_name: text.to_string(),
            }])),
            DispatchParser::UpdateLocalSearchConfigSearch {
                scope,
                if_current_not_found,
                run_search_after_config_updated,
            } => {
                let dispatch = match parse_search_config(text) {
                    Ok(search_config) => match scope {
                        Scope::Local => Dispatch::UpdateLocalSearchConfig {
                            update: LocalSearchConfigUpdate::Config(search_config.local_config),
                            scope,
                            if_current_not_found,
                            run_search_after_config_updated,
                            component_id: None,
                        },
                        Scope::Global => Dispatch::UpdateGlobalSearchConfig {
                            update: GlobalSearchConfigUpdate::Config(search_config),
                        },
                    },
                    Err(error) => Dispatch::ShowEditorInfo(Info::new(
                        "Error".to_string(),
                        format!("{error:?}"),
                    )),
                };
                Ok(Dispatches::one(dispatch)
                    .append(Dispatch::ToEditor(ClearIncrementalSearchMatches)))
            }
            DispatchParser::AddPath => {
                Ok(Dispatches::new([Dispatch::AddPath(text.into())].to_vec()))
            }
            DispatchParser::MovePaths { sources } => Ok(Dispatches::new(
                [Dispatch::MovePaths {
                    sources,
                    destinations: NonEmpty::from_vec(
                        text.lines().map(|line| line.into()).collect_vec(),
                    )
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Failed to move path because there is no destination paths."
                        )
                    })?,
                }]
                .to_vec(),
            )),
            DispatchParser::CopyFile { from } => Ok(Dispatches::new(
                [Dispatch::CopyFile {
                    from,
                    to: text.into(),
                }]
                .to_vec(),
            )),
            #[cfg(test)]
            DispatchParser::SetContent => Ok(Dispatches::new(
                [Dispatch::ToEditor(SetContent(text.to_string()))].to_vec(),
            )),
            DispatchParser::PipeToShell => Ok(Dispatches::one(Dispatch::ToEditor(
                DispatchEditor::PipeToShell {
                    command: text.to_string(),
                },
            ))),
            DispatchParser::FilterSelectionMatchingSearch { maintain } => Ok(Dispatches::one(
                Dispatch::ToEditor(DispatchEditor::FilterSelectionMatchingSearch {
                    maintain,
                    search: text.to_string(),
                }),
            )),
            DispatchParser::SetKeyboardLayoutKind => {
                let keyboard_layout_kind = KeyboardLayoutKind::iter()
                    .find(|keyboard_layout| keyboard_layout.display() == text)
                    .ok_or_else(|| anyhow::anyhow!("No keyboard layout is named {text:?}"))?;
                Ok(Dispatches::one(Dispatch::SetKeyboardLayoutKind(
                    keyboard_layout_kind,
                )))
            }
            DispatchParser::SurroundXmlTag => Ok(Dispatches::one(Dispatch::ToEditor(
                DispatchEditor::Surround(format!("<{text}>"), format!("</{text}>")),
            ))),
            #[cfg(test)]
            DispatchParser::Null => Ok(Default::default()),
            DispatchParser::ChangeWorkingDirectory => Ok(Dispatches::one(
                Dispatch::ChangeWorkingDirectory(text.try_into()?),
            )),
        }
    }
}
