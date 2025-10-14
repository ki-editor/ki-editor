use crate::{
    buffer::{Buffer, BufferOwner},
    clipboard::CopiedTexts,
    components::{
        component::{Component, ComponentId, GetGridResult},
        dropdown::{DropdownItem, DropdownRender},
        editor::{
            Direction, DispatchEditor, Editor, IfCurrentNotFound, Movement, PriorChange, Reveal,
        },
        editor_keymap::{KeyboardLayoutKind, Meaning},
        editor_keymap_printer::KeymapDisplayOption,
        file_explorer::FileExplorer,
        keymap_legend::{Keymap, KeymapLegendConfig, Keymaps},
        prompt::{Prompt, PromptConfig, PromptHistoryKey, PromptItems, PromptItemsBackgroundTask},
        suggestive_editor::{
            DispatchSuggestiveEditor, Info, SuggestiveEditor, SuggestiveEditorFilter,
        },
    },
    context::{
        Context, GlobalMode, GlobalSearchConfig, LocalSearchConfigMode, QuickfixListSource, Search,
    },
    edit::Edit,
    frontend::Frontend,
    git::{self},
    grid::{Grid, LineUpdate},
    integration_event::{IntegrationEvent, IntegrationEventEmitter},
    layout::Layout,
    list::{self, grep::Match, WalkBuilderConfig},
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
    quickfix_list::{Location, QuickfixList, QuickfixListItem, QuickfixListType},
    screen::{Screen, Window},
    search::parse_search_config,
    selection::{CharIndex, SelectionMode},
    syntax_highlight::{HighlightedSpans, SyntaxHighlightRequest, SyntaxHighlightRequestBatchId},
    thread::SendResult,
    ui_tree::{ComponentKind, KindedComponent},
};
use event::event::Event;
use itertools::{Either, Itertools};
use name_variant::NamedVariant;
#[cfg(test)]
use shared::language::LanguageId;
use shared::{canonicalized_path::CanonicalizedPath, language::Language};
use std::sync::Arc;
use std::{
    any::TypeId,
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};
use strum::IntoEnumIterator;
use DispatchEditor::*;

#[cfg(test)]
use crate::layout::BufferContentsMap;

// TODO: rename current Context struct to RawContext struct
// The new Context struct should always be derived, it should contains Hashmap of rectangles, keyed by Component ID
// The scroll offset of each componentn should only be recalculated when:
// 1. The number of components is changed (this means we need to store the components)
// 2. The terminal dimension is changed
pub(crate) struct App<T: Frontend> {
    context: Context,

    sender: Sender<AppMessage>,

    /// Used for receiving message from various sources:
    /// - Events from crossterm
    /// - Notifications from language server
    receiver: Receiver<AppMessage>,

    /// Sender for integration events (used by external integrations like VSCode)
    integration_event_sender: Option<Sender<crate::integration_event::IntegrationEvent>>,

    lsp_manager: LspManager,
    enable_lsp: bool,

    working_directory: CanonicalizedPath,
    global_title: Option<String>,

    layout: Layout,

    frontend: Rc<Mutex<T>>,

    syntax_highlight_request_sender: Option<Sender<SyntaxHighlightRequest>>,
    status_line_components: Vec<StatusLineComponent>,
    last_action_description: Option<String>,
    last_action_short_description: Option<String>,

    /// This is necessary when Ki is running as an embedded application
    last_prompt_config: Option<PromptConfig>,

    /// This is used for suspending events until the buffer content
    /// is synced between Ki and the host application.
    queued_events: Vec<Event>,
}

const GLOBAL_TITLE_BAR_HEIGHT: u16 = 1;

#[derive(Clone)]
pub(crate) enum StatusLineComponent {
    CurrentWorkingDirectory,
    GitBranch,
    Mode,
    SelectionMode,
    LastDispatch,
    LastSearchString,
    Help,
    KeyboardLayout,
    Reveal,
}

impl<T: Frontend> App<T> {
    #[cfg(test)]
    pub(crate) fn new(
        frontend: Rc<Mutex<T>>,
        working_directory: CanonicalizedPath,
        status_line_components: Vec<StatusLineComponent>,
    ) -> anyhow::Result<App<T>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self::from_channel(
            frontend,
            working_directory,
            sender,
            receiver,
            None, // No syntax highlight request sender
            status_line_components,
            None, // No integration event sender
            false,
            false,
            None,
        )
    }

    #[cfg(test)]
    pub(crate) fn disable_lsp(&mut self) {
        self.enable_lsp = false
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_channel(
        frontend: Rc<Mutex<T>>,
        working_directory: CanonicalizedPath,
        sender: Sender<AppMessage>,
        receiver: Receiver<AppMessage>,
        syntax_highlight_request_sender: Option<Sender<SyntaxHighlightRequest>>,
        status_line_components: Vec<StatusLineComponent>,
        integration_event_sender: Option<Sender<crate::integration_event::IntegrationEvent>>,
        enable_lsp: bool,
        is_running_as_embedded: bool,
        persistence: Option<Persistence>,
    ) -> anyhow::Result<App<T>> {
        let dimension = frontend.lock().unwrap().get_terminal_dimension()?;
        let mut app = App {
            context: Context::new(
                working_directory.clone(),
                is_running_as_embedded,
                persistence,
            ),
            receiver,
            lsp_manager: LspManager::new(sender.clone(), working_directory.clone()),
            enable_lsp,
            sender,
            layout: Layout::new(
                dimension.decrement_height(GLOBAL_TITLE_BAR_HEIGHT),
                &working_directory,
            )?,
            working_directory,
            frontend,
            syntax_highlight_request_sender,
            global_title: None,
            status_line_components,
            last_action_description: None,
            last_action_short_description: None,
            integration_event_sender,
            last_prompt_config: None,
            queued_events: Vec::new(),
        };

        app.restore_session();

        Ok(app)
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
    pub(crate) fn run(
        mut self,
        entry_path: Option<CanonicalizedPath>,
    ) -> Result<(), anyhow::Error> {
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
            self.process_message(message).unwrap_or_else(|e| {
                self.show_global_info(Info::new("ERROR".to_string(), e.to_string()));
                false
            });

            if self.should_quit() {
                break;
            }

            self.render()?;
        }

        self.quit()
    }

    pub(crate) fn process_message(&mut self, message: AppMessage) -> anyhow::Result<bool> {
        match message {
            AppMessage::Event(event) => self.handle_event(event),
            AppMessage::LspNotification(notification) => {
                self.handle_lsp_notification(notification).map(|_| false)
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
                self.handle_dispatch(dispatch)?;
                Ok(false)
            }
            AppMessage::NucleoTickDebounced => {
                self.handle_nucleo_debounced()?;
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

    pub(crate) fn quit(&mut self) -> anyhow::Result<()> {
        self.prepare_to_suspend_or_quit()?;

        // self.lsp_manager.shutdown();

        std::process::exit(0);
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

    pub(crate) fn components(&self) -> Vec<KindedComponent> {
        self.layout.components()
    }

    /// Returns true if the app should quit.
    pub(crate) fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        // Pass event to focused window
        let component = self.current_component();
        match event {
            Event::Resize(columns, rows) => {
                self.resize(Dimension {
                    height: rows,
                    width: columns,
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

    pub(crate) fn render(&mut self) -> Result<(), anyhow::Error> {
        let screen = self.get_screen()?;
        self.render_screen(screen)?;
        Ok(())
    }

    fn keyboard_layout_kind(&self) -> &KeyboardLayoutKind {
        self.context.keyboard_layout_kind()
    }

    pub(crate) fn get_screen(&mut self) -> Result<Screen, anyhow::Error> {
        // Recalculate layout before each render
        self.layout.recalculate_layout(&self.context);

        // Generate layout
        let dimension = self.layout.terminal_dimension();
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
                    if cursor_position.line >= rectangle.dimension().height as usize {
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
        let global_title_window = {
            let title = self.global_title.clone().unwrap_or_else(|| {
                let last_search_string = self
                    .context
                    .get_prompt_history(PromptHistoryKey::Search)
                    .last()
                    .map(|search| format!("{search:?}"));
                self.status_line_components
                    .iter()
                    .filter_map(|component| match component {
                        StatusLineComponent::CurrentWorkingDirectory => Some(
                            self.working_directory
                                .display_relative_to_home()
                                .ok()
                                .unwrap_or_else(|| self.working_directory.display_absolute()),
                        ),
                        StatusLineComponent::GitBranch => self.current_branch(),
                        StatusLineComponent::Mode => {
                            let mode = self
                                .context
                                .mode()
                                .map(|mode| mode.display())
                                .unwrap_or_else(|| {
                                    self.current_component().borrow().editor().display_mode()
                                });
                            Some(format!("{mode: <5}"))
                        }
                        StatusLineComponent::SelectionMode => Some(
                            self.current_component()
                                .borrow()
                                .editor()
                                .display_selection_mode(),
                        ),
                        StatusLineComponent::LastDispatch => self.last_action_description.clone(),
                        StatusLineComponent::LastSearchString => last_search_string.clone(),
                        StatusLineComponent::Help => {
                            let key = self.keyboard_layout_kind().get_insert_key(&Meaning::SHelp);
                            Some(format!("Help({key})"))
                        }
                        StatusLineComponent::KeyboardLayout => {
                            Some(self.keyboard_layout_kind().display().to_string())
                        }
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
                            }),
                    })
                    .join(" ")
            });
            let title = format!(" {title}");
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
                        line: dimension.height as usize,
                        column: 0,
                    },
                },
            )
        };
        let screen = screen.add_window(global_title_window);

        Ok(screen)
    }

    fn current_branch(&self) -> Option<String> {
        // Open the repository
        let repo = git2::Repository::open(self.working_directory.display_absolute()).ok()?;

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

    pub(crate) fn handle_dispatches(
        &mut self,
        dispatches: Dispatches,
    ) -> Result<(), anyhow::Error> {
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

    pub(crate) fn handle_dispatch(&mut self, dispatch: Dispatch) -> Result<(), anyhow::Error> {
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
            } => self.open_search_prompt(scope, if_current_not_found)?,
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
            Dispatch::RequestCompletion => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentCompletion(params),
                    )?;
                }
            }
            Dispatch::ResolveCompletionItem(completion_item) => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager.send_message(
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
                    self.lsp_manager.send_message(
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
                    self.lsp_manager
                        .send_message(params.path.clone(), FromEditor::TextDocumentHover(params))?;
                }

                self.send_integration_event(IntegrationEvent::RequestLspHover);
            }
            Dispatch::RequestDefinitions(scope) => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_kind(Some(scope)).set_description("Definitions");
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentDefinition(params),
                    )?;

                    self.send_integration_event(IntegrationEvent::RequestLspDefinition);
                }
            }
            Dispatch::RequestDeclarations(scope) => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_kind(Some(scope)).set_description("Declarations");
                    self.lsp_manager.send_message(
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
                    self.lsp_manager.send_message(
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
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentTypeDefinition(params),
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspTypeDefinition);
                }
            }
            Dispatch::RequestDocumentSymbols => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_description("Document Symbols");
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentDocumentSymbol(params),
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspDocumentSymbols);
                }
            }
            Dispatch::PrepareRename => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentPrepareRename(params),
                    )?;
                    self.send_integration_event(IntegrationEvent::RequestLspRename);
                }
            }
            Dispatch::RenameSymbol { new_name } => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentRename { params, new_name },
                    )?;
                }
            }
            Dispatch::RequestCodeAction { diagnostics } => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager.send_message(
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
                    self.lsp_manager.send_message(
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
                    self.lsp_manager.send_message(
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

                self.lsp_manager.send_message(
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
                self.show_keymap_legend(keymap_legend_config)
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
            Dispatch::SaveQuitAll => self.save_quit_all()?,
            Dispatch::RevealInExplorer(path) => self.reveal_path_in_explorer(&path)?,
            Dispatch::OpenMoveFilePrompt => self.open_move_file_prompt()?,
            Dispatch::OpenDuplicateFilePrompt => self.open_copy_file_prompt()?,
            Dispatch::OpenAddPathPrompt => self.open_add_path_prompt()?,
            Dispatch::OpenDeleteFilePrompt => self.open_delete_file_prompt()?,
            Dispatch::DeletePath(path) => self.delete_path(&path)?,
            Dispatch::Null => {
                // do nothing
            }
            Dispatch::MoveFile { from, to } => self.move_file(from, to)?,
            Dispatch::CopyFile { from, to } => self.copy_file(from, to)?,
            Dispatch::AddPath(path) => self.add_path(path)?,
            Dispatch::RefreshFileExplorer => self
                .layout
                .refresh_file_explorer(&self.working_directory, &self.context)?,
            Dispatch::SetClipboardContent {
                copied_texts: contents,
            } => self.context.set_clipboard_content(contents)?,
            Dispatch::SetGlobalMode(mode) => self.set_global_mode(mode),
            #[cfg(test)]
            Dispatch::HandleKeyEvent(key_event) => {
                self.handle_event(Event::Key(key_event))?;
            }
            Dispatch::GetRepoGitHunks(diff_mode) => self.get_repo_git_hunks(diff_mode)?,
            Dispatch::SaveAll => self.save_all()?,
            #[cfg(test)]
            Dispatch::TerminalDimensionChanged(dimension) => self.resize(dimension),
            #[cfg(test)]
            Dispatch::SetGlobalTitle(title) => self.set_global_title(title),
            Dispatch::LspExecuteCommand { command } => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager.send_message(
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
            } => self.update_local_search_config(
                update,
                scope,
                if_current_not_found,
                run_search_after_config_updated,
            )?,
            Dispatch::UpdateGlobalSearchConfig { update } => {
                self.update_global_search_config(update)?;
            }
            Dispatch::Replace { scope } => match scope {
                Scope::Local => self.handle_dispatch_editor(ReplacePattern {
                    config: self.context.local_search_config().clone(),
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
            Dispatch::OpenPrompt {
                config,
                current_line,
            } => self.open_prompt(config, current_line)?,
            Dispatch::ShowEditorInfo(info) => self.show_editor_info(info)?,
            Dispatch::ReceiveCodeActions(code_actions) => {
                self.open_code_actions_prompt(code_actions)?;
            }
            Dispatch::OtherWindow => self.layout.cycle_window(),
            Dispatch::CycleMarkedFile(direction) => self.cycle_marked_file(direction)?,
            Dispatch::PushPromptHistory { key, line } => self.push_history_prompt(key, line),
            Dispatch::OpenThemePrompt => self.open_theme_prompt()?,
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
            Dispatch::OpenKeyboardLayoutPrompt => self.open_keyboard_layout_prompt()?,
            Dispatch::NavigateForward => self.navigate_forward()?,
            Dispatch::NavigateBack => self.navigate_back()?,
            Dispatch::ToggleFileMark => self.toggle_file_mark()?,
            Dispatch::ToHostApp(to_host_app) => self.handle_to_host_app(to_host_app)?,
            Dispatch::FromHostApp(from_host_app) => self.handle_from_host_app(from_host_app)?,
            Dispatch::OpenSurroundXmlPrompt => self.open_surround_xml_prompt()?,
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
        }
        Ok(())
    }

    pub(crate) fn get_editor_by_file_path(
        &self,
        path: &CanonicalizedPath,
    ) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        self.layout.get_existing_editor(path)
    }

    pub(crate) fn current_component(&self) -> Rc<RefCell<dyn Component>> {
        self.layout.get_current_component()
    }

    fn close_current_window(&mut self) -> anyhow::Result<()> {
        if let Some(removed_path) = self.layout.close_current_window(&self.context) {
            if let Some(path) = self.context.unmark_path(removed_path).cloned() {
                self.open_file(&path, BufferOwner::User, true, true)?;
            }
        }
        Ok(())
    }

    fn local_search(&mut self, if_current_not_found: IfCurrentNotFound) -> anyhow::Result<()> {
        let config = self.context.local_search_config();
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
                self.current_component(),
            )?;
        }

        Ok(())
    }

    fn resize(&mut self, dimension: Dimension) {
        self.layout.set_terminal_dimension(
            dimension.decrement_height(GLOBAL_TITLE_BAR_HEIGHT),
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
        self.open_prompt(
            PromptConfig {
                title: "Move to index".to_string(),
                on_enter: DispatchPrompt::MoveSelectionByIndex,
                items: PromptItems::None,
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::MoveToIndex,
            },
            None,
        )
    }

    fn open_rename_prompt(&mut self, current_name: Option<String>) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: "Rename Symbol".to_string(),
                on_enter: DispatchPrompt::RenameSymbol,
                items: PromptItems::None,
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: false,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::Rename,
            },
            current_name,
        )
    }

    fn open_surround_xml_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: "Surround selection with XML tag (can be empty for React Fragment)"
                    .to_string(),
                on_enter: DispatchPrompt::SurroundXmlTag,
                items: PromptItems::None,
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: false,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::SurroundXmlTag,
            },
            None,
        )
    }

    fn open_search_prompt(
        &mut self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: format!("{scope:?} search",),
                items: PromptItems::Precomputed(self.words()),
                on_enter: DispatchPrompt::UpdateLocalSearchConfigSearch {
                    scope,
                    if_current_not_found,
                    run_search_after_config_updated: true,
                },
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::Search,
            },
            None,
        )
    }

    fn get_file_explorer_current_path(&mut self) -> anyhow::Result<Option<CanonicalizedPath>> {
        self.current_component()
            .borrow_mut()
            .as_any_mut()
            .downcast_mut::<FileExplorer>()
            .and_then(|file_explorer| file_explorer.get_current_path().transpose())
            .transpose()
    }

    fn open_delete_file_prompt(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.get_file_explorer_current_path()? {
            self.open_yes_no_prompt(YesNoPrompt {
                title: format!("Delete \"{}\"?", path.display_absolute()),
                yes: Box::new(Dispatch::DeletePath(path.clone())),
            })
        } else {
            Ok(())
        }
    }

    fn open_add_path_prompt(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.get_file_explorer_current_path()? {
            self.open_prompt(
                PromptConfig {
                    title: "Add path".to_string(),
                    on_enter: DispatchPrompt::AddPath,
                    items: PromptItems::None,
                    enter_selects_first_matching_item: false,
                    leaves_current_line_empty: false,
                    fire_dispatches_on_change: None,
                    prompt_history_key: PromptHistoryKey::AddPath,
                },
                Some(path.display_absolute()),
            )
        } else {
            Ok(())
        }
    }

    fn open_move_file_prompt(&mut self) -> anyhow::Result<()> {
        let path = self.get_file_explorer_current_path()?;
        if let Some(path) = path {
            self.open_prompt(
                PromptConfig {
                    title: "Move path".to_string(),
                    on_enter: DispatchPrompt::MovePath { from: path.clone() },
                    items: PromptItems::None,
                    enter_selects_first_matching_item: false,
                    leaves_current_line_empty: false,
                    fire_dispatches_on_change: None,
                    prompt_history_key: PromptHistoryKey::MovePath,
                },
                Some(path.display_absolute()),
            )
        } else {
            Ok(())
        }
    }

    fn open_copy_file_prompt(&mut self) -> anyhow::Result<()> {
        let path = self.get_file_explorer_current_path()?;
        if let Some(path) = path {
            self.open_prompt(
                PromptConfig {
                    title: format!("Duplicate '{}' to", path.display_absolute()),
                    on_enter: DispatchPrompt::CopyFile { from: path.clone() },
                    items: PromptItems::None,
                    enter_selects_first_matching_item: false,
                    leaves_current_line_empty: false,
                    fire_dispatches_on_change: None,
                    prompt_history_key: PromptHistoryKey::CopyFile,
                },
                Some(path.display_absolute()),
            )
        } else {
            Ok(())
        }
    }

    fn open_symbol_picker(&mut self, symbols: Symbols) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: "Symbols".to_string(),
                items: PromptItems::Precomputed(
                    symbols
                        .symbols
                        .clone()
                        .into_iter()
                        .map(|symbol| symbol.into())
                        .collect_vec(),
                ),
                on_enter: DispatchPrompt::SelectSymbol { symbols },
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::Symbol,
            },
            None,
        )
    }

    fn open_file_picker(&mut self, kind: FilePickerKind) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        self.open_prompt(
            PromptConfig {
                title: format!("Open file: {}", kind.display()),
                on_enter: DispatchPrompt::OpenFile {
                    working_directory: working_directory.clone(),
                },
                items: match kind {
                    FilePickerKind::NonGitIgnored => PromptItems::BackgroundTask {
                        task: PromptItemsBackgroundTask::NonGitIgnoredFiles { working_directory },
                        on_nucleo_tick_debounced: {
                            let sender = self.sender.clone();
                            Arc::new(move || {
                                let _ = sender.send(AppMessage::NucleoTickDebounced);
                            })
                        },
                    },
                    FilePickerKind::GitStatus(diff_mode) => PromptItems::Precomputed(
                        git::GitRepo::try_from(&self.working_directory)?
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
                                DropdownItem::from_path_buf(
                                    &working_directory,
                                    path.into_path_buf(),
                                )
                            })
                            .collect_vec(),
                    ),
                },
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::OpenFile,
            },
            None,
        )
    }

    fn open_file(
        &mut self,
        path: &CanonicalizedPath,
        owner: BufferOwner,
        store_history: bool,
        focus: bool,
    ) -> anyhow::Result<Rc<RefCell<SuggestiveEditor>>> {
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
            self.lsp_manager.open_file(path.clone())?;
        }
        Ok(component)
    }

    pub(crate) fn handle_lsp_notification(
        &mut self,
        notification: LspNotification,
    ) -> anyhow::Result<()> {
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
                        if buffer.borrow().language()? == language {
                            buffer.borrow().path()
                        } else {
                            None
                        }
                    })
                    .collect_vec();
                self.lsp_manager.initialized(language, opened_documents);
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
            LspNotification::Symbols(symbols) => {
                self.open_symbol_picker(symbols)?;
                Ok(())
            }
            LspNotification::CompletionItemResolve(completion_item) => {
                self.update_current_completion_item(completion_item.into())
            }
        }
    }

    pub(crate) fn update_diagnostics(
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

    pub(crate) fn get_quickfix_list(&self) -> Option<QuickfixList> {
        self.context.quickfix_list_state().as_ref().map(|state| {
            let items = self.layout.get_quickfix_list_items(&state.source);
            // Preload the buffers to avoid unnecessarily rereading the files
            let buffers = items
                .iter()
                .map(|item| &item.location().path)
                .unique()
                .filter_map(|path| {
                    Some(Rc::new(RefCell::new(Buffer::from_path(path, false).ok()?)))
                })
                .collect_vec();
            QuickfixList::new(
                state.title.clone(),
                items,
                buffers,
                self.context.current_working_directory(),
            )
            .set_current_item_index(state.current_item_index)
        })
    }

    fn goto_quickfix_list_item(&mut self, movement: Movement) -> anyhow::Result<()> {
        if let Some(mut quickfix_list) = self.get_quickfix_list() {
            if let Some((current_item_index, dispatches)) = quickfix_list.get_item(movement) {
                self.context
                    .set_quickfix_list_current_item_index(current_item_index);
                self.handle_dispatches(dispatches)?;
                self.render_quickfix_list(
                    quickfix_list.set_current_item_index(current_item_index),
                )?;
            } else {
                log::info!("No current item found")
            }
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
        let go_to_first_quickfix = match r#type {
            QuickfixListType::Diagnostic(severity_range) => {
                self.context.set_quickfix_list_source(
                    title.clone(),
                    QuickfixListSource::Diagnostic(severity_range),
                );
                true
            }
            QuickfixListType::Items(items) => {
                let is_empty = items.is_empty();
                self.context
                    .set_quickfix_list_source(title.clone(), QuickfixListSource::Custom(items));
                !is_empty
            }
            QuickfixListType::Mark => {
                self.context
                    .set_quickfix_list_source(title.clone(), QuickfixListSource::Mark);
                true
            }
        };
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
                ResourceOperation::Rename { old, new } => self.move_file(old, new)?,
                ResourceOperation::Delete(path) => self.delete_path(&path)?,
            }
        }
        Ok(())
    }

    fn show_keymap_legend(&mut self, keymap_legend_config: KeymapLegendConfig) {
        if self.is_running_as_embedded() {
            let title = keymap_legend_config.title.clone();
            let body = keymap_legend_config.display(
                self.context.keyboard_layout_kind(),
                u16::MAX,
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
            .show_keymap_legend(keymap_legend_config, &self.context);
        self.layout.recalculate_layout(&self.context);
    }

    fn global_replace(&mut self) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        let global_search_config = self.context.global_search_config();
        let walk_builder_config = WalkBuilderConfig {
            root: working_directory.clone().into(),
            include: global_search_config.include_glob(),
            exclude: global_search_config.exclude_glob(),
        };
        let config = self.context.global_search_config().local_config();
        let affected_paths = list::grep::replace(walk_builder_config, config.clone())?;
        let dispatches = self.layout.reload_buffers(affected_paths)?;
        self.handle_dispatches(dispatches)
    }

    fn global_search(&mut self) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();

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
        let locations = match config.mode {
            LocalSearchConfigMode::Regex(regex) => {
                let sender = self.sender.clone();
                // TODO: we need to create a new sender for each global search, so that it can be cancelled, but when?
                list::grep::run_async(
                    &config.search(),
                    walk_builder_config,
                    regex,
                    Arc::new(move |locations| {
                        SendResult::from(sender.send(AppMessage::ExternalDispatch(
                            Dispatch::AddQuickfixListEntries(locations),
                        )))
                    }),
                )?;
                Ok(Default::default())
            }
            LocalSearchConfigMode::AstGrep => {
                list::ast_grep::run(config.search().clone(), walk_builder_config)
            }
            LocalSearchConfigMode::NamingConventionAgnostic => {
                list::naming_convention_agnostic::run(config.search().clone(), walk_builder_config)
            }
        }?;
        self.set_quickfix_list_type(
            ResponseContext::default().set_description("Global search"),
            QuickfixListType::Items(
                locations
                    .into_iter()
                    .map(|location| QuickfixListItem::new(location, None, None))
                    .collect_vec(),
            ),
        )?;
        Ok(())
    }

    pub(crate) fn quit_all(&self) -> Result<(), anyhow::Error> {
        Ok(self.sender.send(AppMessage::QuitAll)?)
    }

    pub(crate) fn sender(&self) -> Sender<AppMessage> {
        self.sender.clone()
    }

    fn save_quit_all(&mut self) -> anyhow::Result<()> {
        self.save_all()?;
        self.quit_all()?;
        Ok(())
    }

    fn save_all(&self) -> anyhow::Result<()> {
        self.layout.save_all(&self.context)
    }

    fn open_yes_no_prompt(&mut self, prompt: YesNoPrompt) -> anyhow::Result<()> {
        self.handle_dispatch(Dispatch::ShowKeymapLegend(KeymapLegendConfig {
            title: prompt.title.to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    self.keyboard_layout_kind().get_yes_no_key(&Meaning::Yes__),
                    "Yes".to_string(),
                    *prompt.yes,
                ),
                Keymap::new(
                    self.keyboard_layout_kind().get_yes_no_key(&Meaning::No___),
                    "No".to_string(),
                    Dispatch::Null,
                ),
            ]),
        }))
    }

    fn delete_path(&mut self, path: &CanonicalizedPath) -> anyhow::Result<()> {
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        self.layout.remove_suggestive_editor(path);
        self.layout
            .refresh_file_explorer(&self.working_directory, &self.context)?;
        Ok(())
    }

    fn move_file(&mut self, from: CanonicalizedPath, to: PathBuf) -> anyhow::Result<()> {
        use std::fs;
        self.add_path_parent(&to)?;
        fs::rename(from.clone(), to.clone())?;
        self.layout
            .refresh_file_explorer(&self.working_directory, &self.context)?;
        let to = to.try_into()?;

        self.context.rename_path_mark(&from, &to);

        self.reveal_path_in_explorer(&to)?;

        self.lsp_manager.send_message(
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
        self.layout
            .refresh_file_explorer(&self.working_directory, &self.context)?;
        let to = to.try_into()?;
        self.reveal_path_in_explorer(&to)?;
        self.lsp_manager.send_message(
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
        self.layout
            .refresh_file_explorer(&self.working_directory, &self.context)?;
        let path: CanonicalizedPath = path.try_into()?;
        self.reveal_path_in_explorer(&path)?;
        self.lsp_manager.send_message(
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
    pub(crate) fn get_current_selected_texts(&self) -> Vec<String> {
        let _content = self.current_component().borrow().content();
        self.current_component()
            .borrow()
            .editor()
            .get_selected_texts()
    }

    #[cfg(test)]
    pub(crate) fn get_file_content(&self, path: &CanonicalizedPath) -> String {
        self.layout
            .get_existing_editor(path)
            .unwrap()
            .borrow()
            .content()
    }

    pub(crate) fn handle_dispatch_editor(
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
                .append(component.borrow().editor().dispatch_marks_changed())
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
        let working_directory = self.working_directory.clone();
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

    pub(crate) fn get_current_file_path(&self) -> Option<CanonicalizedPath> {
        self.current_component().borrow().path()
    }

    fn set_global_mode(&mut self, mode: Option<GlobalMode>) {
        self.context.set_mode(mode);
    }

    #[cfg(test)]
    pub(crate) fn context(&self) -> &Context {
        &self.context
    }

    fn update_local_search_config(
        &mut self,
        update: LocalSearchConfigUpdate,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        run_search_after_config_updated: bool,
    ) -> Result<(), anyhow::Error> {
        self.context.update_local_search_config(update, scope);
        if run_search_after_config_updated {
            match scope {
                Scope::Local => self.local_search(if_current_not_found)?,
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

    fn cycle_marked_file(&mut self, direction: Direction) -> anyhow::Result<()> {
        if let Some(next_file_path) = {
            let file_paths = self.context.get_marked_files();
            self.get_current_file_path()
                .and_then(|current_file_path| {
                    if let Some(current_index) = file_paths
                        .iter()
                        .position(|path| path == &&current_file_path)
                    {
                        let next_index = match direction {
                            Direction::Start if current_index == 0 => file_paths.len() - 1,
                            Direction::Start => current_index - 1,
                            Direction::End if current_index == file_paths.len() - 1 => 0,
                            Direction::End => current_index + 1,
                        };
                        // We are doing defensive programming here
                        // to ensure that Ki editor never crashes
                        return file_paths.get(next_index);
                    }
                    None
                })
                .or_else(|| file_paths.first())
                .cloned()
        } {
            if next_file_path.exists() {
                self.open_file(&next_file_path.clone(), BufferOwner::User, true, true)?;
            } else {
                // If the file no longer exists, remove it from the list of marked files
                // and then cycle to the next file
                self.context.toggle_path_mark(next_file_path.clone());
                self.cycle_marked_file(direction)?
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn get_current_component_content(&self) -> String {
        self.current_component().borrow().editor().content()
    }

    #[cfg(test)]
    pub(crate) fn get_buffer_contents_map(&self) -> BufferContentsMap {
        self.layout.get_buffer_contents_map()
    }

    #[cfg(test)]
    fn handle_key_events(&mut self, key_events: Vec<event::KeyEvent>) -> anyhow::Result<()> {
        for key_event in key_events.into_iter() {
            self.handle_event(Event::Key(key_event.to_owned()))?;
        }
        Ok(())
    }

    pub(crate) fn handle_dispatch_suggestive_editor(
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
            Err(anyhow::anyhow!(
                "The current component is neither Prompt or SuggestiveEditor, thus `App::handle_dispatch_suggestive_editor` does nothing."
            ))
        }
    }

    #[cfg(test)]
    pub(crate) fn completion_dropdown_is_open(&self) -> bool {
        self.layout.completion_dropdown_is_open()
    }

    pub(crate) fn current_completion_dropdown(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.current_completion_dropdown()
    }

    fn open_prompt(
        &mut self,
        prompt_config: PromptConfig,
        current_line: Option<String>,
    ) -> anyhow::Result<()> {
        if self.is_running_as_embedded() {
            self.open_prompt_embedded(prompt_config, current_line)
        } else {
            self.open_prompt_non_embedded(prompt_config, current_line)
        }
    }

    fn open_prompt_non_embedded(
        &mut self,
        prompt_config: PromptConfig,
        current_line: Option<String>,
    ) -> anyhow::Result<()> {
        if let Some(line) = current_line {
            self.context
                .push_history_prompt(prompt_config.prompt_history_key, line)
        }
        let key = prompt_config.prompt_history_key;
        let history = self.context.get_prompt_history(key);
        let (prompt, dispatches) = Prompt::new(prompt_config, history);

        self.layout.add_and_focus_prompt(
            ComponentKind::Prompt,
            Rc::new(RefCell::new(prompt)),
            &self.context,
        );
        self.handle_dispatches(dispatches)
    }

    fn open_prompt_embedded(
        &mut self,
        prompt_config: PromptConfig,
        current_line: Option<String>,
    ) -> anyhow::Result<()> {
        let key = prompt_config.prompt_history_key;

        let history = self.context.get_prompt_history(key);

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

        if let Some(line) = current_line {
            self.context.push_history_prompt(key, line)
        }
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
        let dispatches = prompt_config.on_enter.to_dispatches(&entry)?;
        self.handle_dispatches(dispatches.append(Dispatch::PushPromptHistory {
            key: prompt_config.prompt_history_key,
            line: entry,
        }))
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
    pub(crate) fn get_dropdown_infos_count(&self) -> usize {
        self.layout.get_dropdown_infos_count()
    }

    pub(crate) fn render_quickfix_list(
        &mut self,
        quickfix_list: QuickfixList,
    ) -> anyhow::Result<()> {
        let dispatches = self
            .layout
            .show_quickfix_list(quickfix_list, &self.context)?;
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
    pub(crate) fn editor_info_contents(&self) -> Vec<String> {
        self.layout.editor_info_contents()
    }

    #[cfg(test)]
    pub(crate) fn global_info_contents(&self) -> Vec<String> {
        self.layout.global_info_contents()
    }

    fn reveal_path_in_explorer(&mut self, path: &CanonicalizedPath) -> anyhow::Result<()> {
        let dispatches = self.layout.reveal_path_in_explorer(path, &self.context)?;
        self.handle_dispatches(dispatches)
    }

    #[cfg(test)]
    pub(crate) fn file_explorer_content(&self) -> String {
        self.layout.file_explorer_content()
    }

    fn open_code_actions_prompt(
        &mut self,
        code_actions: Vec<crate::lsp::code_action::CodeAction>,
    ) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                on_enter: DispatchPrompt::Null,
                items: PromptItems::Precomputed(
                    code_actions
                        .into_iter()
                        .map(move |code_action| code_action.into())
                        .collect(),
                ),
                title: "Code Actions".to_string(),
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::CodeAction,
            },
            None,
        )?;
        Ok(())
    }

    fn close_current_window_and_focus_parent(&mut self) {
        self.layout.close_current_window_and_focus_parent();
        self.integration_event_sender
            .emit_event(IntegrationEvent::ShowInfo { info: None });
    }

    pub(crate) fn opened_files_count(&self) -> usize {
        self.layout.get_opened_files().len()
    }

    #[cfg(test)]
    pub(crate) fn global_info(&self) -> Option<String> {
        self.layout.global_info()
    }

    #[cfg(test)]
    pub(crate) fn get_component_by_kind(
        &self,
        kind: ComponentKind,
    ) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.get_component_by_kind(kind)
    }

    fn hide_editor_info(&mut self) {
        self.layout.hide_editor_info()
    }

    #[cfg(test)]
    pub(crate) fn components_order(&self) -> Vec<ComponentKind> {
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

    fn open_theme_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                on_enter: DispatchPrompt::Null,
                items: PromptItems::Precomputed(
                    crate::themes::theme_descriptor::all()
                        .into_iter()
                        .enumerate()
                        .map(|(index, theme_descriptor)| {
                            DropdownItem::new(theme_descriptor.name().to_string())
                                .set_rank(Some(Box::from([index].to_vec())))
                                .set_dispatches(Dispatches::one(Dispatch::SetThemeFromDescriptor(
                                    theme_descriptor,
                                )))
                        })
                        .collect_vec(),
                ),
                title: "Theme".to_string(),
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: Some(Dispatches::one(Dispatch::SetTheme(
                    self.context.theme().clone(),
                ))),
                prompt_history_key: PromptHistoryKey::Theme,
            },
            None,
        )
    }

    fn open_keyboard_layout_prompt(&mut self) -> anyhow::Result<()> {
        let embedded = self.context.is_running_as_embedded();
        self.open_prompt(
            PromptConfig {
                on_enter: if embedded {
                    DispatchPrompt::SetKeyboardLayoutKind
                } else {
                    DispatchPrompt::Null
                },
                items: PromptItems::Precomputed(
                    KeyboardLayoutKind::iter()
                        .map(|keyboard_layout| {
                            DropdownItem::new(keyboard_layout.display().to_string()).set_dispatches(
                                if embedded {
                                    Dispatches::default()
                                } else {
                                    Dispatches::one(Dispatch::SetKeyboardLayoutKind(
                                        keyboard_layout,
                                    ))
                                },
                            )
                        })
                        .collect_vec(),
                ),
                title: "Keyboard Layout".to_string(),
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::KeyboardLayout,
            },
            None,
        )
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
    pub(crate) fn lsp_request_sent(&self, from_editor: &FromEditor) -> bool {
        self.lsp_manager.lsp_request_sent(from_editor)
    }

    fn open_pipe_to_shell_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: "Pipe to shell".to_string(),
                items: PromptItems::None,
                on_enter: DispatchPrompt::PipeToShell,
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::PipeToShell,
            },
            None,
        )
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
        let config = self.context.get_local_search_config(Scope::Local);
        let mode = config.mode;
        self.open_prompt(
            PromptConfig {
                title: format!(
                    "{} selections matching search ({})",
                    if maintain { "Maintain" } else { "Remove" },
                    mode.display()
                ),
                on_enter: DispatchPrompt::FilterSelectionMatchingSearch { maintain },
                items: PromptItems::None,
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
                prompt_history_key: PromptHistoryKey::FilterSelectionsMatchingSearch { maintain },
            },
            None,
        )
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

    pub(crate) fn take_queued_events(&mut self) -> Vec<Event> {
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
        self.open_search_prompt(scope, if_current_not_found)?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn lsp_server_initialized_args(
        &self,
    ) -> Option<(LanguageId, Vec<CanonicalizedPath>)> {
        self.lsp_manager.lsp_server_initialized_args()
    }

    fn handle_nucleo_debounced(&mut self) -> Result<(), anyhow::Error> {
        let dispatches = {
            let component = self.layout.get_current_component();
            let mut component_mut = component.borrow_mut();
            let Some(prompt) = component_mut.as_any_mut().downcast_mut::<Prompt>() else {
                return Ok(());
            };

            let viewport_height: u32 = self
                .current_completion_dropdown()
                .map(|component| component.borrow().rectangle().height)
                .unwrap_or(10)
                .into();

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
            let _ = self.cycle_marked_file(Direction::End);
        }
    }

    fn add_quickfix_list_entries(&mut self, matches: Vec<Match>) -> anyhow::Result<()> {
        let go_to_quickfix_item = self.context.quickfix_list_items().is_empty();

        log::info!("go_to_quickix_item = {go_to_quickfix_item}");

        self.context.extend_quickfix_list_items(
            matches
                .into_iter()
                .map(|m| QuickfixListItem::new(m.location, None, Some(m.line)))
                .collect_vec(),
        );

        let quickfix_list = self.get_quickfix_list();
        if let Some(quickfix_list) = quickfix_list {
            self.render_quickfix_list(quickfix_list)?;
        }
        if go_to_quickfix_item {
            self.goto_quickfix_list_item(Movement::Current(IfCurrentNotFound::LookForward))?;
        }
        Ok(())
    }

    fn handle_applied_edits(&mut self, path: CanonicalizedPath, edits: Vec<Edit>) {
        self.context.handle_applied_edits(path, edits)
    }

    #[cfg(test)]
    pub(crate) fn handle_next_app_message(&mut self) -> anyhow::Result<()> {
        use std::time::Duration;

        std::thread::sleep(Duration::from_secs(1));
        match self.receiver.try_recv() {
            Ok(app_message) => {
                self.process_message(app_message)?;
            }
            Err(err) => eprintln!("App::handle_next_app_message: {err:?}"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Dimension {
    pub(crate) height: u16,
    pub(crate) width: u16,
}

impl Dimension {
    #[cfg(test)]
    pub(crate) fn area(&self) -> usize {
        self.height as usize * self.width as usize
    }

    #[cfg(test)]
    pub(crate) fn positions(&self) -> std::collections::HashSet<Position> {
        (0..self.height as usize)
            .flat_map(|line| (0..self.width as usize).map(move |column| Position { column, line }))
            .collect()
    }

    fn decrement_height(&self, global_title_bar_height: u16) -> Dimension {
        Dimension {
            height: self.height.saturating_sub(global_title_bar_height),
            width: self.width,
        }
    }
}

#[must_use]
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Dispatches(Vec<Dispatch>);
impl From<Vec<Dispatch>> for Dispatches {
    fn from(value: Vec<Dispatch>) -> Self {
        Self(value)
    }
}
impl Dispatches {
    pub(crate) fn into_vec(self) -> Vec<Dispatch> {
        self.0
    }

    pub(crate) fn new(dispatches: Vec<Dispatch>) -> Dispatches {
        Dispatches(dispatches)
    }

    pub(crate) fn chain(self, other: Dispatches) -> Dispatches {
        self.0.into_iter().chain(other.0).collect_vec().into()
    }

    pub(crate) fn append(self, other: Dispatch) -> Dispatches {
        self.0.into_iter().chain(Some(other)).collect_vec().into()
    }

    pub(crate) fn append_some(self, dispatch: Option<Dispatch>) -> Dispatches {
        if let Some(dispatch) = dispatch {
            self.append(dispatch)
        } else {
            self
        }
    }

    pub(crate) fn one(edit: Dispatch) -> Dispatches {
        Dispatches(vec![edit])
    }

    pub(crate) fn empty() -> Dispatches {
        Dispatches(Default::default())
    }
}

#[must_use]
#[derive(Clone, Debug, PartialEq, NamedVariant)]
/// Dispatch are for child component to request action from the root node
pub(crate) enum Dispatch {
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
    RemainOnlyCurrentComponent,

    #[cfg(test)]
    /// Used for testing
    Custom(String),
    ToEditor(DispatchEditor),
    RequestDocumentSymbols,
    GotoLocation(Location),
    OpenMoveToIndexPrompt(Option<PriorChange>),
    QuitAll,
    SaveQuitAll,
    RevealInExplorer(CanonicalizedPath),
    OpenMoveFilePrompt,
    OpenDuplicateFilePrompt,
    OpenAddPathPrompt,
    DeletePath(CanonicalizedPath),
    Null,
    MoveFile {
        from: CanonicalizedPath,
        to: PathBuf,
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
        current_line: Option<String>,
    },
    ShowEditorInfo(Info),
    ReceiveCodeActions(Vec<crate::lsp::code_action::CodeAction>),
    OtherWindow,
    CloseCurrentWindowAndFocusParent,
    CloseEditorInfo,
    CloseGlobalInfo,
    CycleMarkedFile(Direction),
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
    OpenDeleteFilePrompt,
    SelectCompletionItem,
    SetKeyboardLayoutKind(KeyboardLayoutKind),
    OpenKeyboardLayoutPrompt,
    NavigateForward,
    NavigateBack,
    ToggleFileMark,
    Suspend,

    ToHostApp(ToHostApp),
    FromHostApp(FromHostApp),
    OpenSurroundXmlPrompt,
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
}

/// Used to send notify host app about changes
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ToHostApp {
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
pub(crate) enum FromHostApp {
    TargetedEvent {
        event: Event,
        path: Option<CanonicalizedPath>,
        content_hash: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum GlobalSearchConfigUpdate {
    Config(GlobalSearchConfig),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LocalSearchConfigUpdate {
    #[cfg(test)]
    Mode(LocalSearchConfigMode),
    #[cfg(test)]
    Replacement(String),
    #[cfg(test)]
    Search(String),
    Config(crate::context::LocalSearchConfig),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct YesNoPrompt {
    pub(crate) title: String,
    pub(crate) yes: Box<Dispatch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FilePickerKind {
    NonGitIgnored,
    GitStatus(git::DiffMode),
    Opened,
}
impl FilePickerKind {
    pub(crate) fn display(&self) -> String {
        match self {
            FilePickerKind::NonGitIgnored => "Not Git Ignored".to_string(),
            FilePickerKind::GitStatus(diff_mode) => format!("Git Status ({})", diff_mode.display()),
            FilePickerKind::Opened => "Opened".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestParams {
    pub(crate) path: CanonicalizedPath,
    pub(crate) position: Position,
    pub(crate) context: ResponseContext,
}
impl RequestParams {
    pub(crate) fn set_kind(self, scope: Option<Scope>) -> Self {
        Self {
            context: ResponseContext {
                scope,
                ..self.context
            },
            ..self
        }
    }

    pub(crate) fn set_description(self, description: &str) -> Self {
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
pub(crate) enum Scope {
    Local,
    Global,
}

#[derive(Debug)]
pub(crate) enum AppMessage {
    LspNotification(LspNotification),
    Event(Event),
    QuitAll,
    SyntaxHighlightResponse {
        component_id: ComponentId,
        batch_id: SyntaxHighlightRequestBatchId,
        highlighted_spans: HighlightedSpans,
    },
    // New variant for external dispatches
    ExternalDispatch(Dispatch),
    NucleoTickDebounced,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DispatchPrompt {
    MoveSelectionByIndex,
    RenameSymbol,
    UpdateLocalSearchConfigSearch {
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        run_search_after_config_updated: bool,
    },
    AddPath,
    MovePath {
        from: CanonicalizedPath,
    },
    CopyFile {
        from: CanonicalizedPath,
    },
    Null,
    // TODO: remove the following variants
    // Because the following action already embeds dispatches
    SelectSymbol {
        symbols: Symbols,
    },
    OpenFile {
        working_directory: CanonicalizedPath,
    },
    #[cfg(test)]
    SetContent,
    PipeToShell,
    FilterSelectionMatchingSearch {
        maintain: bool,
    },
    SetKeyboardLayoutKind,
    SurroundXmlTag,
}
impl DispatchPrompt {
    pub(crate) fn to_dispatches(&self, text: &str) -> anyhow::Result<Dispatches> {
        match self.clone() {
            DispatchPrompt::MoveSelectionByIndex => {
                let index = text.parse::<usize>()?.saturating_sub(1);
                Ok(Dispatches::new(
                    [Dispatch::ToEditor(MoveSelection(Movement::Index(index)))].to_vec(),
                ))
            }
            DispatchPrompt::RenameSymbol => Ok(Dispatches::new(vec![Dispatch::RenameSymbol {
                new_name: text.to_string(),
            }])),
            DispatchPrompt::UpdateLocalSearchConfigSearch {
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
                Ok(Dispatches::one(dispatch))
            }
            DispatchPrompt::AddPath => {
                Ok(Dispatches::new([Dispatch::AddPath(text.into())].to_vec()))
            }
            DispatchPrompt::MovePath { from } => Ok(Dispatches::new(
                [Dispatch::MoveFile {
                    from,
                    to: text.into(),
                }]
                .to_vec(),
            )),
            DispatchPrompt::CopyFile { from } => Ok(Dispatches::new(
                [Dispatch::CopyFile {
                    from,
                    to: text.into(),
                }]
                .to_vec(),
            )),
            DispatchPrompt::SelectSymbol { symbols } => {
                // TODO: make Prompt generic over the item type,
                // so that we don't have to do this,
                // i.e. we can just return the symbol directly,
                // instead of having to find it again.
                if let Some(symbol) = symbols
                    .symbols
                    .iter()
                    .find(|symbol| text == symbol.display())
                {
                    Ok(Dispatches::new(vec![Dispatch::GotoLocation(
                        symbol.location.clone(),
                    )]))
                } else {
                    Ok(Dispatches::new(vec![]))
                }
            }
            DispatchPrompt::OpenFile { working_directory } => {
                let path = working_directory.join(text)?;
                Ok(Dispatches::new(vec![Dispatch::OpenFile {
                    path,
                    owner: BufferOwner::User,
                    focus: true,
                }]))
            }
            #[cfg(test)]
            DispatchPrompt::SetContent => Ok(Dispatches::new(
                [Dispatch::ToEditor(SetContent(text.to_string()))].to_vec(),
            )),
            DispatchPrompt::Null => Ok(Default::default()),
            DispatchPrompt::PipeToShell => Ok(Dispatches::one(Dispatch::ToEditor(
                DispatchEditor::PipeToShell {
                    command: text.to_string(),
                },
            ))),
            DispatchPrompt::FilterSelectionMatchingSearch { maintain } => Ok(Dispatches::one(
                Dispatch::ToEditor(DispatchEditor::FilterSelectionMatchingSearch {
                    maintain,
                    search: text.to_string(),
                }),
            )),
            DispatchPrompt::SetKeyboardLayoutKind => {
                let keyboard_layout_kind = KeyboardLayoutKind::iter()
                    .find(|keyboard_layout| keyboard_layout.display() == text)
                    .ok_or_else(|| anyhow::anyhow!("No keyboard layout is named {text:?}"))?;
                Ok(Dispatches::one(Dispatch::SetKeyboardLayoutKind(
                    keyboard_layout_kind,
                )))
            }
            DispatchPrompt::SurroundXmlTag => Ok(Dispatches::one(Dispatch::ToEditor(
                DispatchEditor::Surround(format!("<{text}>"), format!("</{text}>")),
            ))),
        }
    }
}
