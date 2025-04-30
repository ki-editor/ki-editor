use crate::{
    buffer::{Buffer, BufferOwner},
    clipboard::CopiedTexts,
    components::{
        component::{Component, ComponentId, GetGridResult},
        dropdown::{DropdownItem, DropdownRender},
        editor::{
            Direction, DispatchEditor, Editor, IfCurrentNotFound, Movement, Reveal, ViewAlignment,
        },
        editor_keymap::{KeyboardLayoutKind, Meaning},
        file_explorer::FileExplorer,
        keymap_legend::{Keymap, KeymapLegendBody, KeymapLegendConfig, Keymaps},
        prompt::{Prompt, PromptConfig, PromptHistoryKey},
        suggestive_editor::{
            DispatchSuggestiveEditor, Info, SuggestiveEditor, SuggestiveEditorFilter,
        },
    },
    context::{Context, GlobalMode, LocalSearchConfigMode, QuickfixListSource, Search},
    frontend::Frontend,
    git,
    grid::{Grid, LineUpdate},
    layout::Layout,
    list::{self, grep::RegexConfig, WalkBuilderConfig},
    lsp::{
        completion::CompletionItem,
        goto_definition_response::GotoDefinitionResponse,
        manager::LspManager,
        process::{FromEditor, LspNotification, ResponseContext},
        symbols::Symbols,
        workspace_edit::WorkspaceEdit,
    },
    position::Position,
    quickfix_list::{Location, QuickfixList, QuickfixListItem, QuickfixListType},
    render_flex_layout::{self, FlexLayoutComponent},
    screen::{Screen, Window},
    selection::SelectionMode,
    syntax_highlight::{HighlightedSpans, SyntaxHighlightRequest, SyntaxHighlightRequestBatchId},
    ui_tree::{ComponentKind, KindedComponent},
};
use event::event::Event;
use itertools::{Either, Itertools};
use name_variant::NamedVariant;
use shared::{canonicalized_path::CanonicalizedPath, language::Language};
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

pub(crate) struct App<T: Frontend> {
    context: Context,

    sender: Sender<AppMessage>,

    /// Used for receiving message from various sources:
    /// - Events from crossterm
    /// - Notifications from language server
    receiver: Receiver<AppMessage>,

    lsp_manager: LspManager,
    enable_lsp: bool,

    working_directory: CanonicalizedPath,
    global_title: Option<String>,

    layout: Layout,

    frontend: Rc<Mutex<T>>,

    syntax_highlight_request_sender: Option<Sender<SyntaxHighlightRequest>>,

    status_lines: Vec<StatusLine>,
    last_action_description: Option<String>,
    last_action_short_description: Option<String>,
}

#[derive(Clone)]
pub(crate) struct StatusLine {
    components: Vec<StatusLineComponent>,
}
impl StatusLine {
    pub(crate) fn new(components: Vec<StatusLineComponent>) -> Self {
        Self { components }
    }
}

#[derive(Clone)]
pub(crate) enum StatusLineComponent {
    ProcessId,
    CurrentWorkingDirectory,
    GitBranch,
    Mode,
    SelectionMode,
    LastDispatch,
    LineColumn,
    LocalSearchConfig,
    Help,
    KeyboardLayout,
    ViewAlignment,
    Reveal,
    Spacer,
}

impl<T: Frontend> App<T> {
    #[cfg(test)]
    pub(crate) fn new(
        frontend: Rc<Mutex<T>>,
        working_directory: CanonicalizedPath,
        status_lines: Vec<StatusLine>,
    ) -> anyhow::Result<App<T>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self::from_channel(frontend, working_directory, sender, receiver, status_lines)
    }

    #[cfg(test)]
    pub(crate) fn disable_lsp(&mut self) {
        self.enable_lsp = false
    }

    pub(crate) fn from_channel(
        frontend: Rc<Mutex<T>>,
        working_directory: CanonicalizedPath,
        sender: Sender<AppMessage>,
        receiver: Receiver<AppMessage>,
        status_lines: Vec<StatusLine>,
    ) -> anyhow::Result<App<T>> {
        let dimension = frontend.lock().unwrap().get_terminal_dimension()?;
        let app = App {
            context: Context::new(working_directory.clone()),
            receiver,
            lsp_manager: LspManager::new(sender.clone(), working_directory.clone()),
            enable_lsp: true,
            sender,
            layout: Layout::new(
                dimension.decrement_height(status_lines.len() as u16),
                &working_directory,
            )?,
            working_directory,
            frontend,
            syntax_highlight_request_sender: None,
            global_title: None,
            status_lines,
            last_action_description: None,
            last_action_short_description: None,
        };
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

    pub(crate) fn run(
        mut self,
        entry_path: Option<CanonicalizedPath>,
    ) -> Result<(), anyhow::Error> {
        {
            let mut frontend = self.frontend.lock().unwrap();
            frontend.enter_alternate_screen()?;
            frontend.enable_raw_mode()?;
            frontend.enable_mouse_capture()?;
        }

        if let Some(entry_path) = entry_path {
            if entry_path.as_ref().is_dir() {
                self.layout.open_file_explorer();
            } else {
                self.open_file(&entry_path, BufferOwner::User, true, true)?;
            }
        }

        self.render()?;

        while let Ok(message) = self.receiver.recv() {
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
            }
            .unwrap_or_else(|e| {
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

    pub(crate) fn quit(&mut self) -> anyhow::Result<()> {
        let mut frontend = self.frontend.lock().unwrap();
        frontend.leave_alternate_screen()?;
        frontend.disable_raw_mode()?;
        frontend.disable_mouse_capture()?;
        // self.lsp_manager.shutdown();

        std::process::exit(0);
    }

    pub(crate) fn components(&self) -> Vec<KindedComponent> {
        self.layout.components()
    }

    /// Returns true if the app should quit.
    fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
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
                    .borrow()
                    .get_grid(&self.context, focused);
                let cursor_position = if focused {
                    if let Some(cursor) = cursor {
                        let cursor_position = cursor.position();
                        // If cursor position is not in view
                        if cursor_position.line >= (rectangle.dimension().height) as usize {
                            None
                        } else {
                            let calibrated_position = Position::new(
                                cursor_position.line + rectangle.origin.line,
                                cursor_position.column + rectangle.origin.column,
                            );
                            Some(cursor.set_position(calibrated_position))
                        }
                    } else {
                        None
                    }
                } else {
                    None
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
            let width = (dimension.width as usize)
                .saturating_sub(leading_padding)
                .saturating_sub(1); // This is the extra space for rendering cursor at the last column
            render_flex_layout::render_flex_layout(
                width,
                separator,
                &status_line
                    .components
                    .iter()
                    .filter_map(|component| match component {
                        StatusLineComponent::CurrentWorkingDirectory => {
                            Some(FlexLayoutComponent::Text({
                                let cwd = self
                                    .working_directory
                                    .display_relative_to_home()
                                    .ok()
                                    .unwrap_or_else(|| self.working_directory.display_absolute());
                                self.current_branch()
                                    .map(|branch| format!("{cwd} ({branch})"))
                                    .unwrap_or(cwd)
                            }))
                        }
                        StatusLineComponent::GitBranch => {
                            self.current_branch().map(FlexLayoutComponent::Text)
                        }
                        StatusLineComponent::Mode => Some(FlexLayoutComponent::Text(
                            self.context
                                .mode()
                                .map(|mode| mode.display())
                                .unwrap_or_else(|| {
                                    self.current_component().borrow().editor().display_mode()
                                }),
                        )),
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
                        StatusLineComponent::LocalSearchConfig => Some(FlexLayoutComponent::Text(
                            self.context.local_search_config().display(),
                        )),
                        StatusLineComponent::Help => Some(FlexLayoutComponent::Text({
                            let key = self.keyboard_layout_kind().get_insert_key(&Meaning::SHelp);
                            format!("Help ({key})")
                        })),
                        StatusLineComponent::KeyboardLayout => Some(FlexLayoutComponent::Text(
                            self.keyboard_layout_kind().display().to_string(),
                        )),
                        StatusLineComponent::ViewAlignment => Some(FlexLayoutComponent::Text(
                            match self
                                .current_component()
                                .borrow()
                                .editor()
                                .current_view_alignment
                            {
                                Some(ViewAlignment::Top) => "↑️",
                                Some(ViewAlignment::Center) | None => "↕️",
                                Some(ViewAlignment::Bottom) => "↓️",
                            }
                            .to_string(),
                        )),
                        StatusLineComponent::Reveal => self
                            .current_component()
                            .borrow()
                            .editor()
                            .reveal()
                            .map(|split| {
                                match split {
                                    Reveal::CurrentSelectionMode => "÷ Selection",
                                    Reveal::Cursor => "÷ Cursor",
                                    Reveal::Mark => "÷ Mark",
                                }
                                .to_string()
                            })
                            .map(FlexLayoutComponent::Text),
                        StatusLineComponent::Spacer => Some(FlexLayoutComponent::Spacer),
                        StatusLineComponent::ProcessId => Some(FlexLayoutComponent::Text(format!(
                            "PID:{}",
                            std::process::id()
                        ))),
                        StatusLineComponent::LineColumn => {
                            let primary_selection_range = self
                                .current_component()
                                .borrow()
                                .editor()
                                .editor()
                                .selection_set
                                .primary_selection()
                                .extended_range();
                            fn format_position(position: Position) -> String {
                                format!("{}:{}", position.line + 1, position.column + 1)
                            }
                            self.current_component()
                                .borrow()
                                .editor()
                                .buffer()
                                .char_index_range_to_position_range(primary_selection_range)
                                .ok()
                                .map(|range| {
                                    FlexLayoutComponent::Text(if range.start == range.end {
                                        format_position(range.start)
                                    } else if range.start.line == range.end.line {
                                        format!(
                                            "{}-{}",
                                            format_position(range.start),
                                            range.end.column
                                        )
                                    } else {
                                        format!(
                                            "{}-{}",
                                            format_position(range.start),
                                            format_position(range.end)
                                        )
                                    })
                                })
                        }
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
        );
        Window::new(
            grid,
            crate::rectangle::Rectangle {
                width: dimension.width,
                height: 1,
                origin: Position {
                    line: (dimension.height as usize) + index,
                    column: 0,
                },
            },
        )
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

    pub(crate) fn handle_dispatch(&mut self, dispatch: Dispatch) -> Result<(), anyhow::Error> {
        log::info!("App::handle_dispatch = {}", dispatch.variant_name());
        match dispatch {
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
            Dispatch::OpenPipeToShellPrompt => self.open_pipe_to_shell_prompt()?,
            Dispatch::OpenFile { path, owner, focus } => {
                self.open_file(&path, owner, true, focus)?;
            }
            Dispatch::OpenFileFromPathBuf { path, owner, focus } => {
                self.open_file(&path.try_into()?, owner, true, focus)?;
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
                            completion_item,
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
                }
            }
            Dispatch::RequestHover => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_description("Hover");
                    self.lsp_manager
                        .send_message(params.path.clone(), FromEditor::TextDocumentHover(params))?;
                }
            }
            Dispatch::RequestDefinitions(scope) => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_kind(Some(scope)).set_description("Definitions");
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentDefinition(params),
                    )?;
                }
            }
            Dispatch::RequestDeclarations(scope) => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_kind(Some(scope)).set_description("Declarations");
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentDeclaration(params),
                    )?;
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
                }
            }
            Dispatch::RequestDocumentSymbols => {
                if let Some(params) = self.get_request_params() {
                    let params = params.set_description("Document Symbols");
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentDocumentSymbol(params),
                    )?;
                }
            }
            Dispatch::PrepareRename => {
                if let Some(params) = self.get_request_params() {
                    self.lsp_manager.send_message(
                        params.path.clone(),
                        FromEditor::TextDocumentPrepareRename(params),
                    )?;
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
                if let Some(path) = path {
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
            Dispatch::OpenMoveToIndexPrompt => self.open_move_to_index_prompt()?,
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
                use_system_clipboard,
            } => self
                .context
                .set_clipboard_content(contents, use_system_clipboard)?,
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
                show_config_after_enter,
                if_current_not_found,
                run_search_after_config_updated,
            } => self.update_local_search_config(
                update,
                scope,
                show_config_after_enter,
                if_current_not_found,
                run_search_after_config_updated,
            )?,
            Dispatch::UpdateGlobalSearchConfig {
                update,
                if_current_not_found,
            } => {
                self.update_global_search_config(update, if_current_not_found)?;
            }
            Dispatch::OpenSetGlobalSearchFilterGlobPrompt {
                filter_glob,
                if_current_not_found,
            } => {
                self.open_set_global_search_filter_glob_prompt(filter_glob, if_current_not_found)?
            }
            Dispatch::ShowSearchConfig {
                scope,
                if_current_not_found,
                run_search_after_config_updated,
            } => self.show_search_config(
                scope,
                if_current_not_found,
                run_search_after_config_updated,
            ),
            Dispatch::OpenUpdateReplacementPrompt {
                scope,
                if_current_not_found,
            } => self.open_update_replacement_prompt(scope, if_current_not_found)?,
            Dispatch::OpenUpdateSearchPrompt {
                scope,
                if_current_not_found,
            } => self.open_update_search_prompt(scope, if_current_not_found)?,
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
                let context = std::mem::take(&mut self.context);
                self.context = context.set_theme(theme.clone());
            }
            Dispatch::SetThemeFromDescriptor(theme_descriptor) => {
                let context = std::mem::take(&mut self.context);
                self.context = context.set_theme(theme_descriptor.to_theme());
            }
            #[cfg(test)]
            Dispatch::HandleKeyEvents(key_events) => self.handle_key_events(key_events)?,
            Dispatch::CloseDropdown => self.layout.close_dropdown(),
            Dispatch::CloseEditorInfo => self.layout.close_editor_info(),
            Dispatch::RenderDropdown { render } => {
                if let Some(dropdown) = self.layout.open_dropdown(&self.context) {
                    self.render_dropdown(dropdown, render)?;
                }
            }
            #[cfg(test)]
            Dispatch::OpenPrompt {
                config,
                key,
                current_line,
            } => self.open_prompt(config, key, current_line)?,
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
                self.context.set_keyboard_layout_kind(keyboard_layout_kind)
            }
            Dispatch::OpenKeyboardLayoutPrompt => self.open_keyboard_layout_prompt()?,
            Dispatch::NavigateForward => self.navigate_forward()?,
            Dispatch::NavigateBack => self.navigate_back()?,
            Dispatch::ToggleFileMark => self.toggle_file_mark()?,
        }
        Ok(())
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
            dimension.decrement_height(self.global_title_bar_height() as u16),
            &self.context,
        );
    }

    fn open_move_to_index_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: "Move to index".to_string(),
                on_enter: DispatchPrompt::MoveSelectionByIndex,
                items: vec![],
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::MoveToIndex,
            None,
        )
    }

    fn open_rename_prompt(&mut self, current_name: Option<String>) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: "Rename Symbol".to_string(),
                on_enter: DispatchPrompt::RenameSymbol,
                items: vec![],
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: false,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::Rename,
            current_name,
        )
    }

    fn open_search_prompt(
        &mut self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: format!(
                    "{:?} search (config search = {})",
                    scope,
                    self.keyboard_layout_kind().get_key(&Meaning::CSrch)
                ),
                items: self.words(),
                on_enter: DispatchPrompt::UpdateLocalSearchConfigSearch {
                    scope,
                    show_config_after_enter: false,
                    if_current_not_found,
                    run_search_after_config_updated: true,
                },
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::Search(scope),
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
                    items: Vec::new(),
                    enter_selects_first_matching_item: false,
                    leaves_current_line_empty: false,
                    fire_dispatches_on_change: None,
                },
                PromptHistoryKey::AddPath,
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
                    items: Vec::new(),
                    enter_selects_first_matching_item: false,
                    leaves_current_line_empty: false,
                    fire_dispatches_on_change: None,
                },
                PromptHistoryKey::MovePath,
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
                    items: Vec::new(),
                    enter_selects_first_matching_item: false,
                    leaves_current_line_empty: false,
                    fire_dispatches_on_change: None,
                },
                PromptHistoryKey::CopyFile,
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
                items: symbols
                    .symbols
                    .clone()
                    .into_iter()
                    .map(|symbol| symbol.into())
                    .collect_vec(),
                on_enter: DispatchPrompt::SelectSymbol { symbols },
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::Symbol,
            None,
        )
    }

    fn open_file_picker(&mut self, kind: FilePickerKind) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        self.open_prompt(
            PromptConfig {
                title: format!("Open file: {}", kind.display()),
                on_enter: DispatchPrompt::OpenFile { working_directory },
                items: {
                    match kind {
                        FilePickerKind::NonGitIgnored => {
                            // Note: we should not use CanonicalizedPath here, as it is resource-intensive
                            list::WalkBuilderConfig::non_git_ignored_files(
                                self.working_directory.clone(),
                            )?
                        }
                        FilePickerKind::GitStatus(diff_mode) => {
                            git::GitRepo::try_from(&self.working_directory)?
                                .diff_entries(diff_mode)?
                                .into_iter()
                                .map(|entry| entry.new_path().into_path_buf())
                                .collect_vec()
                        }
                        FilePickerKind::Opened => self
                            .layout
                            .get_opened_files()
                            .into_iter()
                            .map(|path| path.into_path_buf())
                            .collect_vec(),
                    }
                    .into_iter()
                    .map(|path| {
                        DropdownItem::new({
                            let name = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let icon = shared::canonicalized_path::get_path_icon(&path);
                            format!("{icon} {name}")
                        })
                        .set_group(path.parent().map(|parent| {
                            let relative = parent
                                .strip_prefix(&self.working_directory)
                                .map(|path| path.display().to_string())
                                .unwrap_or_else(|_| parent.display().to_string());
                            format!("{} {}", shared::icons::get_icon_config().folder, relative,)
                        }))
                        .set_dispatches(Dispatches::one(
                            crate::app::Dispatch::OpenFileFromPathBuf {
                                path,
                                owner: BufferOwner::User,
                                focus: true,
                            },
                        ))
                    })
                    .collect_vec()
                },
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::OpenFile,
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
            self.push_current_location_into_navigation_history(true)?;
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
                .replace_and_focus_current_suggestive_editor(component.clone())
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
                self.lsp_manager.initialized(
                    language,
                    self.layout
                        .buffers()
                        .into_iter()
                        .filter_map(|buffer| buffer.borrow().path())
                        .collect_vec(),
                );
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

    fn update_diagnostics(
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
            QuickfixList::new(
                state.title.clone(),
                self.layout.get_quickfix_list_items(&state.source),
                self.layout.buffers(),
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
            }
        }

        Ok(())
    }

    fn show_global_info(&mut self, info: Info) {
        self.layout
            .show_global_info(info, &self.context)
            .unwrap_or_else(|err| {
                log::error!("Error showing info: {:?}", err);
            });
    }

    fn go_to_location(
        &mut self,
        location: &Location,
        store_history: bool,
    ) -> Result<(), anyhow::Error> {
        let component = self.open_file(&location.path, BufferOwner::System, store_history, true)?;
        let dispatches = component
            .borrow_mut()
            .editor_mut()
            .set_position_range(location.range.clone(), &self.context)?;
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
        match r#type {
            QuickfixListType::Diagnostic(severity_range) => {
                self.context.set_quickfix_list_source(
                    title.clone(),
                    QuickfixListSource::Diagnostic(severity_range),
                );
            }
            QuickfixListType::Items(items) => {
                self.layout.clear_quickfix_list_items();
                items
                    .into_iter()
                    .chunk_by(|item| item.location().path.clone())
                    .into_iter()
                    .sorted_by_key(|(path, _)| path.clone())
                    .map(|(path, items)| -> anyhow::Result<()> {
                        let items = items.collect_vec();
                        let editor = self.open_file(&path, BufferOwner::System, false, false)?;
                        editor
                            .borrow_mut()
                            .editor_mut()
                            .buffer_mut()
                            .update_quickfix_list_items(items);
                        Ok(())
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                self.context
                    .set_quickfix_list_source(title.clone(), QuickfixListSource::Custom);
            }
            QuickfixListType::Mark => {
                self.context
                    .set_quickfix_list_source(title.clone(), QuickfixListSource::Mark);
            }
        }
        match context.scope {
            None | Some(Scope::Global) => {
                self.goto_quickfix_list_item(Movement::Current(IfCurrentNotFound::LookForward))?;
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
        self.layout
            .show_keymap_legend(keymap_legend_config, &self.context)
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
        self.layout.reload_buffers(affected_paths)
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
                list::grep::run(&config.search(), walk_builder_config, regex)
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
                    .map(|location| QuickfixListItem::new(location, None))
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
            body: KeymapLegendBody::Positional(Keymaps::new(&[
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
            ])),
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
        let dispatches = component
            .borrow_mut()
            .handle_dispatch_editor(&mut self.context, dispatch_editor)?;

        self.handle_dispatches(dispatches)?;
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
                            .map(|hunk| {
                                let line_range = hunk.line_range();
                                let location = Location {
                                    path: file_diff.path().clone(),
                                    range: Position {
                                        line: line_range.start,
                                        column: 0,
                                    }..Position {
                                        line: line_range.end,
                                        column: 0,
                                    },
                                };
                                QuickfixListItem::new(location, hunk.to_info())
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

    pub(crate) fn set_syntax_highlight_request_sender(
        &mut self,
        sender: Sender<SyntaxHighlightRequest>,
    ) {
        self.syntax_highlight_request_sender = Some(sender);
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
        show_legend: bool,
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

        if show_legend {
            self.show_search_config(scope, if_current_not_found, run_search_after_config_updated);
        }
        Ok(())
    }

    fn update_global_search_config(
        &mut self,
        update: GlobalSearchConfigUpdate,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<()> {
        self.context.update_global_search_config(update)?;
        self.global_search()?;
        self.show_search_config(Scope::Global, if_current_not_found, true);
        Ok(())
    }

    fn open_set_global_search_filter_glob_prompt(
        &mut self,
        filter_glob: GlobalSearchFilterGlob,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                title: format!("Set global search {:?} files glob", filter_glob),
                on_enter: DispatchPrompt::GlobalSearchConfigSetGlob {
                    filter_glob,
                    if_current_not_found,
                },
                items: Vec::new(),
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: false,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::FilterGlob(filter_glob),
            None,
        )
    }

    fn show_search_config(
        &mut self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        run_search_after_config_updated: bool,
    ) {
        fn show_checkbox(title: &str, checked: bool) -> String {
            format!("[{}] {title}", if checked { "X" } else { " " })
        }
        let global_search_confing = match scope {
            Scope::Local => None,
            Scope::Global => Some(self.context.global_search_config()),
        };
        let local_search_config = global_search_confing
            .map(|config| config.local_config())
            .unwrap_or_else(|| self.context.local_search_config());
        let update_keymap =
            |key: &'static str, description: String, update: LocalSearchConfigUpdate| -> Keymap {
                Keymap::new(
                    key,
                    description,
                    Dispatch::UpdateLocalSearchConfig {
                        update,
                        scope,
                        show_config_after_enter: scope == Scope::Global,
                        if_current_not_found: IfCurrentNotFound::LookForward,
                        run_search_after_config_updated,
                    },
                )
            };
        let update_mode_keymap = |key: &'static str,
                                  name: String,
                                  mode: LocalSearchConfigMode,
                                  checked: bool|
         -> Keymap {
            let description = show_checkbox(&name, checked);
            update_keymap(key, description, LocalSearchConfigUpdate::Mode(mode))
        };
        let regex = match local_search_config.mode {
            LocalSearchConfigMode::Regex(regex) => Some(regex),
            LocalSearchConfigMode::AstGrep => None,
            LocalSearchConfigMode::NamingConventionAgnostic => None,
        };
        self.show_keymap_legend(KeymapLegendConfig {
            title: format!("Configure Search ({:?})", scope),
            body: KeymapLegendBody::Mnemonic(Keymaps::new(
                &[
                    Keymap::new(
                        self.keyboard_layout_kind()
                            .get_search_config_keymap(&Meaning::Srch_),
                        format!("Search = {}", local_search_config.search()),
                        Dispatch::OpenUpdateSearchPrompt {
                            scope,
                            if_current_not_found,
                        },
                    ),
                    Keymap::new(
                        self.keyboard_layout_kind()
                            .get_search_config_keymap(&Meaning::Rplcm),
                        format!("Replacement = {}", local_search_config.replacement()),
                        Dispatch::OpenUpdateReplacementPrompt {
                            scope,
                            if_current_not_found,
                        },
                    ),
                    update_mode_keymap(
                        self.keyboard_layout_kind()
                            .get_search_config_keymap(&Meaning::ASTGp),
                        "AST Grep".to_string(),
                        LocalSearchConfigMode::AstGrep,
                        local_search_config.mode == LocalSearchConfigMode::AstGrep,
                    ),
                    update_mode_keymap(
                        self.keyboard_layout_kind()
                            .get_search_config_keymap(&Meaning::NCAgn),
                        "Naming Convention Agnostic".to_string(),
                        LocalSearchConfigMode::NamingConventionAgnostic,
                        local_search_config.mode == LocalSearchConfigMode::NamingConventionAgnostic,
                    ),
                    update_mode_keymap(
                        self.keyboard_layout_kind()
                            .get_search_config_keymap(&Meaning::Litrl),
                        "Literal".to_string(),
                        LocalSearchConfigMode::Regex(RegexConfig {
                            escaped: true,
                            ..regex.unwrap_or_default()
                        }),
                        regex.map(|regex| regex.escaped).unwrap_or(false),
                    ),
                    update_mode_keymap(
                        self.keyboard_layout_kind()
                            .get_search_config_keymap(&Meaning::Regex),
                        "Regex".to_string(),
                        LocalSearchConfigMode::Regex(RegexConfig {
                            escaped: false,
                            ..regex.unwrap_or_default()
                        }),
                        regex.map(|regex| !regex.escaped).unwrap_or(false),
                    ),
                    Keymap::new(
                        self.keyboard_layout_kind()
                            .get_search_config_keymap(&Meaning::RplcA),
                        "Replace all".to_string(),
                        Dispatch::Replace { scope },
                    ),
                ]
                .into_iter()
                .chain(
                    regex
                        .map(|regex| {
                            [
                                update_mode_keymap(
                                    self.keyboard_layout_kind()
                                        .get_search_config_keymap(&Meaning::CaStv),
                                    "Case-sensitive".to_string(),
                                    LocalSearchConfigMode::Regex(RegexConfig {
                                        case_sensitive: !regex.case_sensitive,
                                        ..regex
                                    }),
                                    regex.case_sensitive,
                                ),
                                update_mode_keymap(
                                    self.keyboard_layout_kind()
                                        .get_search_config_keymap(&Meaning::Strct),
                                    "Strict".to_string(),
                                    LocalSearchConfigMode::Regex(RegexConfig {
                                        match_whole_word: true,
                                        case_sensitive: true,
                                        ..regex
                                    }),
                                    regex.match_whole_word && regex.case_sensitive,
                                ),
                                update_mode_keymap(
                                    self.keyboard_layout_kind()
                                        .get_search_config_keymap(&Meaning::Flexi),
                                    "Flexible".to_string(),
                                    LocalSearchConfigMode::Regex(RegexConfig {
                                        match_whole_word: false,
                                        case_sensitive: false,
                                        ..regex
                                    }),
                                    !regex.match_whole_word && !regex.case_sensitive,
                                ),
                                update_mode_keymap(
                                    self.keyboard_layout_kind()
                                        .get_search_config_keymap(&Meaning::MaWWd),
                                    "Match whole word".to_string(),
                                    LocalSearchConfigMode::Regex(RegexConfig {
                                        match_whole_word: !regex.match_whole_word,
                                        ..regex
                                    }),
                                    regex.match_whole_word,
                                ),
                            ]
                            .to_vec()
                        })
                        .unwrap_or_default(),
                )
                .chain(
                    global_search_confing
                        .map(|config| {
                            [
                                Keymap::new(
                                    self.keyboard_layout_kind()
                                        .get_search_config_keymap(&Meaning::InFGb),
                                    format!(
                                        "Include files (glob) = {}",
                                        config
                                            .include_glob()
                                            .map(|glob| glob.to_string())
                                            .unwrap_or_default()
                                    ),
                                    Dispatch::OpenSetGlobalSearchFilterGlobPrompt {
                                        filter_glob: GlobalSearchFilterGlob::Include,
                                        if_current_not_found,
                                    },
                                ),
                                Keymap::new(
                                    self.keyboard_layout_kind()
                                        .get_search_config_keymap(&Meaning::ExFGb),
                                    format!(
                                        "Exclude files (glob) = {}",
                                        config
                                            .exclude_glob()
                                            .map(|glob| glob.to_string())
                                            .unwrap_or_default()
                                    ),
                                    Dispatch::OpenSetGlobalSearchFilterGlobPrompt {
                                        filter_glob: GlobalSearchFilterGlob::Exclude,
                                        if_current_not_found,
                                    },
                                ),
                            ]
                            .to_vec()
                        })
                        .unwrap_or_default(),
                )
                .collect_vec(),
            )),
        })
    }

    fn open_update_replacement_prompt(
        &mut self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> Result<(), anyhow::Error> {
        self.open_prompt(
            PromptConfig {
                title: format!("Set Replace ({:?})", scope),
                on_enter: DispatchPrompt::UpdateLocalSearchConfigReplacement {
                    scope,
                    if_current_not_found,
                },
                items: Vec::new(),
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: false,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::Replacement(scope),
            None,
        )
    }

    fn open_update_search_prompt(
        &mut self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> Result<(), anyhow::Error> {
        self.open_prompt(
            PromptConfig {
                title: format!("Set Search ({:?})", scope),
                on_enter: DispatchPrompt::UpdateLocalSearchConfigSearch {
                    scope,
                    show_config_after_enter: true,
                    if_current_not_found,
                    run_search_after_config_updated: true,
                },
                items: self.words(),
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: false,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::Search(scope),
            None,
        )
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
            let file_paths = self.context.get_marked_paths();
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
            self.open_file(&next_file_path.clone(), BufferOwner::User, true, true)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn get_current_component_content(&self) -> String {
        self.current_component().borrow().editor().content()
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

    #[cfg(test)]
    pub(crate) fn current_completion_dropdown(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.current_completion_dropdown()
    }

    fn open_prompt(
        &mut self,
        prompt_config: PromptConfig,
        key: PromptHistoryKey,
        current_line: Option<String>,
    ) -> anyhow::Result<()> {
        let history = self.context.get_prompt_history(key, current_line);
        let (prompt, dispatches) = Prompt::new(prompt_config, key, history);

        self.layout.add_and_focus_prompt(
            ComponentKind::Prompt,
            Rc::new(RefCell::new(prompt)),
            &self.context,
        );
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
        self.layout.show_editor_info(info, &self.context)
    }

    #[cfg(test)]
    pub(crate) fn editor_info_open(&self) -> bool {
        self.layout.editor_info_open()
    }

    #[cfg(test)]
    pub(crate) fn editor_info_content(&self) -> Option<String> {
        self.layout.editor_info_content()
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
                items: code_actions
                    .into_iter()
                    .map(move |code_action| code_action.into())
                    .collect(),
                title: "Code Actions".to_string(),
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::CodeAction,
            None,
        )?;
        Ok(())
    }

    fn close_current_window_and_focus_parent(&mut self) {
        self.layout.close_current_window_and_focus_parent()
    }

    #[cfg(test)]
    pub(crate) fn opened_files_count(&self) -> usize {
        self.layout.get_opened_files().len()
    }

    #[cfg(test)]
    pub(crate) fn quickfix_list_info(&self) -> Option<String> {
        self.layout.quickfix_list_info()
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
                items: crate::themes::theme_descriptor::all()
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
                title: "Theme".to_string(),
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: Some(Dispatches::one(Dispatch::SetTheme(
                    self.context.theme().clone(),
                ))),
            },
            PromptHistoryKey::Theme,
            None,
        )
    }

    fn open_keyboard_layout_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(
            PromptConfig {
                on_enter: DispatchPrompt::Null,
                items: KeyboardLayoutKind::iter()
                    .map(|keyboard_layout| {
                        DropdownItem::new(keyboard_layout.display().to_string())
                            .set_info(Some(Info::new(
                                "Layout".to_string(),
                                keyboard_layout
                                    .get_keyboard_layout()
                                    .into_iter()
                                    .map(|row| row.join(" "))
                                    .join("\n"),
                            )))
                            .set_dispatches(Dispatches::one(Dispatch::SetKeyboardLayoutKind(
                                keyboard_layout,
                            )))
                    })
                    .collect_vec(),
                title: "Keyboard Layout".to_string(),
                enter_selects_first_matching_item: true,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::Theme,
            None,
        )
    }

    fn update_current_completion_item(
        &mut self,
        completion_item: CompletionItem,
    ) -> anyhow::Result<()> {
        self.handle_dispatch_suggestive_editor(
            DispatchSuggestiveEditor::UpdateCurrentCompletionItem(completion_item),
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
                items: Default::default(),
                on_enter: DispatchPrompt::PipeToShell,
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::PipeToShell,
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
                items: Vec::new(),
                enter_selects_first_matching_item: false,
                leaves_current_line_empty: true,
                fire_dispatches_on_change: None,
            },
            PromptHistoryKey::FilterSelectionsMatchingSearch { maintain },
            None,
        )
    }

    fn navigate_back(&mut self) -> anyhow::Result<()> {
        if let Some(location) = self.context.location_previous() {
            self.push_current_location_into_navigation_history(false)?;
            self.go_to_location(&location, false)?
        }
        Ok(())
    }

    fn navigate_forward(&mut self) -> anyhow::Result<()> {
        if let Some(location) = self.context.location_next() {
            self.push_current_location_into_navigation_history(true)?;
            self.go_to_location(&location, false)?
        }
        Ok(())
    }

    fn push_current_location_into_navigation_history(
        &mut self,
        backward: bool,
    ) -> anyhow::Result<()> {
        // TODO: should include scroll offset as well
        // so that when the user navigates back, it really feels the same
        if let Some(path) = self.current_component().borrow().editor().path() {
            let range = self
                .current_component()
                .borrow()
                .editor()
                .current_selection_range()?;
            let location = Location { path, range };
            self.context.push_location_history(location, backward)
        }
        Ok(())
    }

    fn toggle_file_mark(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.get_current_file_path() {
            if let Some(new_path) = self.context.toggle_file_mark(path).cloned() {
                self.open_file(&new_path, BufferOwner::User, true, true)?;
            }
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
    OpenMoveToIndexPrompt,
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
        use_system_clipboard: bool,
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
        show_config_after_enter: bool,
        if_current_not_found: IfCurrentNotFound,
        run_search_after_config_updated: bool,
    },
    UpdateGlobalSearchConfig {
        update: GlobalSearchConfigUpdate,
        if_current_not_found: IfCurrentNotFound,
    },
    OpenSetGlobalSearchFilterGlobPrompt {
        filter_glob: GlobalSearchFilterGlob,
        if_current_not_found: IfCurrentNotFound,
    },
    ShowSearchConfig {
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        run_search_after_config_updated: bool,
    },
    OpenUpdateReplacementPrompt {
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    },
    OpenUpdateSearchPrompt {
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
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
        key: PromptHistoryKey,
        current_line: Option<String>,
    },
    ShowEditorInfo(Info),
    ReceiveCodeActions(Vec<crate::lsp::code_action::CodeAction>),
    OtherWindow,
    CloseCurrentWindowAndFocusParent,
    CloseEditorInfo,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum GlobalSearchConfigUpdate {
    SetGlob(GlobalSearchFilterGlob, String),
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, Copy)]
pub(crate) enum GlobalSearchFilterGlob {
    Include,
    Exclude,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LocalSearchConfigUpdate {
    Mode(LocalSearchConfigMode),
    Replacement(String),
    Search(String),
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DispatchPrompt {
    GlobalSearchConfigSetGlob {
        filter_glob: GlobalSearchFilterGlob,
        if_current_not_found: IfCurrentNotFound,
    },
    MoveSelectionByIndex,
    RenameSymbol,
    UpdateLocalSearchConfigSearch {
        scope: Scope,
        show_config_after_enter: bool,
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
    UpdateLocalSearchConfigReplacement {
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    },
    #[cfg(test)]
    SetContent,
    PipeToShell,
    FilterSelectionMatchingSearch {
        maintain: bool,
    },
}
impl DispatchPrompt {
    pub(crate) fn to_dispatches(&self, text: &str) -> anyhow::Result<Dispatches> {
        match self.clone() {
            DispatchPrompt::GlobalSearchConfigSetGlob {
                filter_glob,
                if_current_not_found,
            } => Ok(Dispatches::new(
                [Dispatch::UpdateGlobalSearchConfig {
                    update: GlobalSearchConfigUpdate::SetGlob(filter_glob, text.to_string()),
                    if_current_not_found,
                }]
                .to_vec(),
            )),
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
                show_config_after_enter,
                if_current_not_found,
                run_search_after_config_updated,
            } => Ok(Dispatches::new(
                [Dispatch::UpdateLocalSearchConfig {
                    update: LocalSearchConfigUpdate::Search(text.to_string()),
                    scope,
                    show_config_after_enter,
                    if_current_not_found,
                    run_search_after_config_updated,
                }]
                .to_vec(),
            )),
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
            DispatchPrompt::UpdateLocalSearchConfigReplacement {
                scope,
                if_current_not_found,
            } => Ok(Dispatches::new(
                [Dispatch::UpdateLocalSearchConfig {
                    scope,
                    update: LocalSearchConfigUpdate::Replacement(text.to_owned()),
                    show_config_after_enter: true,
                    if_current_not_found,
                    run_search_after_config_updated: true,
                }]
                .to_vec(),
            )),
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
        }
    }
}
