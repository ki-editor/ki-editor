use event::event::Event;
use itertools::Itertools;
use my_proc_macros::key;
use shared::{canonicalized_path::CanonicalizedPath, language::Language};
use std::{
    cell::RefCell,
    collections::HashSet,
    path::PathBuf,
    rc::Rc,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

use crate::{
    buffer::Buffer,
    components::{
        component::{Component, ComponentId, Cursor, GetGridResult},
        editor::{DispatchEditor, Movement},
        keymap_legend::{
            Keymap, KeymapLegendBody, KeymapLegendConfig, KeymapLegendSection, Keymaps,
        },
        prompt::{Prompt, PromptConfig},
        suggestive_editor::{Info, SuggestiveEditor, SuggestiveEditorFilter},
    },
    context::{Context, GlobalMode, LocalSearchConfigMode, Search},
    frontend::frontend::Frontend,
    git,
    grid::Grid,
    layout::Layout,
    list::{self, grep::RegexConfig, WalkBuilderConfig},
    lsp::{
        completion::CompletionItem,
        diagnostic::Diagnostic,
        goto_definition_response::GotoDefinitionResponse,
        manager::LspManager,
        process::{LspNotification, ResponseContext},
        symbols::Symbols,
        workspace_edit::WorkspaceEdit,
    },
    position::Position,
    quickfix_list::{Location, QuickfixList, QuickfixListItem, QuickfixListType},
    selection::{Filter, FilterKind, FilterMechanism, FilterTarget, SelectionMode, SelectionSet},
    selection_mode::inside::InsideKind,
    syntax_highlight::{HighlighedSpans, SyntaxHighlightRequest},
    themes::VSCODE_LIGHT,
    undo_tree::{Applicable, UndoTree},
};

pub struct App<T: Frontend> {
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

    frontend: Arc<Mutex<T>>,

    syntax_highlight_request_sender: Option<Sender<SyntaxHighlightRequest>>,

    undo_tree: UndoTree<FileSelectionSet>,
}

#[derive(PartialEq, Clone, Debug)]
struct FileSelectionSet {
    path: CanonicalizedPath,
    selection_set: SelectionSet,
}

impl std::fmt::Display for FileSelectionSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{}:{}",
            self.path.try_display_relative(),
            self.selection_set.primary.extended_range().start.0
        ))
    }
}

struct Null;
impl std::fmt::Display for Null {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NULL")
    }
}

impl Applicable for FileSelectionSet {
    type Target = Layout;

    type Output = Null;

    fn apply(&self, target: &mut Self::Target) -> anyhow::Result<Self::Output> {
        target.open_file_with_selection(&self.path, self.selection_set.clone())?;
        Ok(Null)
    }
}

const GLOBAL_TITLE_BAR_HEIGHT: u16 = 1;
impl<T: Frontend> App<T> {
    pub fn new(
        frontend: Arc<Mutex<T>>,
        working_directory: CanonicalizedPath,
    ) -> anyhow::Result<App<T>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self::from_channel(frontend, working_directory, sender, receiver)
    }

    pub fn disable_lsp(&mut self) {
        self.enable_lsp = false
    }

    pub fn from_channel(
        frontend: Arc<Mutex<T>>,
        working_directory: CanonicalizedPath,
        sender: Sender<AppMessage>,
        receiver: Receiver<AppMessage>,
    ) -> anyhow::Result<App<T>> {
        let dimension = frontend.lock().unwrap().get_terminal_dimension()?;
        let app = App {
            context: Context::new(working_directory.clone()),
            receiver,
            lsp_manager: LspManager::new(sender.clone(), working_directory.clone()),
            enable_lsp: true,
            sender,
            layout: Layout::new(
                dimension.decrement_height(GLOBAL_TITLE_BAR_HEIGHT),
                &working_directory,
            )?,
            working_directory,
            frontend,
            syntax_highlight_request_sender: None,
            global_title: None,
            undo_tree: UndoTree::new(),
        };
        Ok(app)
    }
    fn update_highlighted_spans(
        &self,
        component_id: ComponentId,
        highlighted_spans: HighlighedSpans,
    ) -> Result<(), anyhow::Error> {
        self.layout
            .update_highlighted_spans(component_id, highlighted_spans)
    }

    pub fn run(mut self, entry_path: Option<CanonicalizedPath>) -> Result<(), anyhow::Error> {
        {
            let mut frontend = self.frontend.lock().unwrap();
            frontend.enter_alternate_screen()?;
            frontend.enable_raw_mode()?;
            frontend.enable_mouse_capture()?;
        }

        if let Some(entry_path) = entry_path {
            self.open_file(&entry_path, true)?;
        } else {
            self.open_file_picker(FilePickerKind::NonGitIgnored)
                .unwrap_or_else(|_| self.layout.open_file_explorer());
        }

        self.render()?;

        while let Ok(message) = self.receiver.recv() {
            let should_quit = match message {
                AppMessage::Event(event) => self.handle_event(event),
                AppMessage::LspNotification(notification) => {
                    self.handle_lsp_notification(notification).map(|_| false)
                }
                AppMessage::QuitAll => Ok(true),
                AppMessage::SyntaxHighlightResponse {
                    component_id,
                    highlighted_spans,
                } => self
                    .update_highlighted_spans(component_id, highlighted_spans)
                    .map(|_| false),
            }
            .unwrap_or_else(|e| {
                self.show_info("ERROR", Info::new(e.to_string()));
                false
            });

            if should_quit {
                break;
            }

            self.render()?;
        }

        let mut frontend = self.frontend.lock().unwrap();
        frontend.leave_alternate_screen()?;
        frontend.disable_raw_mode()?;
        // self.lsp_manager.shutdown();

        // TODO: this line is a hack
        std::process::exit(0);

        Ok(())
    }

    pub fn components(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        self.layout.components()
    }

    /// Returns true if the app should quit.
    fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        // Pass event to focused window
        let component = self.current_component();
        match event {
            Event::Key(key!("enter")) if self.context.mode().is_some() => {
                self.context.set_mode(None);
            }
            Event::Key(key!("ctrl+q")) => {
                if self.quit() {
                    return Ok(true);
                }
            }
            Event::Key(key!("ctrl+w")) => self.layout.change_view(),
            Event::Resize(columns, rows) => {
                self.resize(Dimension {
                    height: rows,
                    width: columns,
                });
            }
            event => {
                component.map(|component| {
                    let dispatches = component.borrow_mut().handle_event(&self.context, event);
                    self.handle_dispatches_result(dispatches)
                        .unwrap_or_else(|e| self.show_info("ERROR", Info::new(e.to_string())));
                });
            }
        }

        Ok(false)
    }

    /// Return true if there's no more windows
    fn quit(&mut self) -> bool {
        self.layout.remove_current_component()
    }

    fn render(&mut self) -> Result<(), anyhow::Error> {
        let GetGridResult { grid, cursor } = self.get_grid()?;
        self.render_grid(grid, cursor)?;
        Ok(())
    }

    pub fn get_grid(&mut self) -> Result<GetGridResult, anyhow::Error> {
        // Recalculate layout before each render
        self.layout.recalculate_layout();

        // Generate layout
        let dimension = self.layout.terminal_dimension();
        let grid = Grid::new(Dimension {
            height: dimension.height,
            width: dimension.width,
        });

        // Render every window
        let (grid, cursor) = self
            .components()
            .into_iter()
            .map(|component| {
                let component = component.borrow();

                let rectangle = component.rectangle();
                let GetGridResult { grid, cursor } = component.get_grid(&mut self.context);
                let focused_component_id = self.layout.focused_component_id();
                let cursor_position = if focused_component_id
                    .map(|focused_component_id| component.id() == focused_component_id)
                    .unwrap_or(false)
                {
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

                (grid, rectangle.clone(), cursor_position)
            })
            .fold(
                (grid, None),
                |(grid, current_cursor_point), (component_grid, rectangle, cursor_point)| {
                    {
                        (
                            grid.update(&component_grid, &rectangle),
                            current_cursor_point.or(cursor_point),
                        )
                    }
                },
            );

        // Render every border
        let grid = self
            .layout
            .borders()
            .into_iter()
            .fold(grid, Grid::set_border);

        // Set the global title
        let global_title_grid = {
            let mode = self.context.mode().map(|mode| mode.display()).or_else(|| {
                self.current_component()
                    .map(|component| component.borrow().editor().display_mode())
            });

            let mode = if let Some(mode) = mode {
                format!("[{}]", mode)
            } else {
                String::new()
            };

            let title = if let Some(title) = self.global_title.as_ref() {
                title.clone()
            } else {
                let branch = if let Some(current_branch) = self.current_branch() {
                    format!(" ({}) ", current_branch,)
                } else {
                    " ".to_string()
                };
                format!(
                    "{}{}{}",
                    self.working_directory.display_absolute(),
                    branch,
                    mode
                )
            };

            Grid::new(Dimension {
                height: 1,
                width: dimension.width,
            })
            .set_line(0, &title, &self.context.theme().ui.global_title)
        };
        let grid = grid.merge_vertical(global_title_grid);

        Ok(GetGridResult { grid, cursor })
    }

    fn current_branch(&self) -> Option<String> {
        // Open the repository
        let repo = git2::Repository::open(self.working_directory.display_absolute()).ok()?;

        // Get the current branch
        let head = repo.head().ok()?;
        let branch = head.shorthand()?;
        Some(branch.to_string())
    }

    fn render_grid(&mut self, grid: Grid, cursor: Option<Cursor>) -> Result<(), anyhow::Error> {
        let mut frontend = self.frontend.lock().unwrap();
        frontend.hide_cursor()?;
        frontend.render_grid(grid)?;
        if let Some(position) = cursor {
            frontend.show_cursor(&position)?;
        }

        Ok(())
    }

    fn handle_dispatches_result(
        &mut self,
        dispatches: anyhow::Result<Vec<Dispatch>>,
    ) -> anyhow::Result<()> {
        self.handle_dispatches(dispatches?)
    }

    pub fn handle_dispatches(&mut self, dispatches: Vec<Dispatch>) -> Result<(), anyhow::Error> {
        for dispatch in dispatches {
            self.handle_dispatch(dispatch)?;
        }
        Ok(())
    }

    pub fn handle_dispatch(&mut self, dispatch: Dispatch) -> Result<(), anyhow::Error> {
        match dispatch {
            Dispatch::CloseCurrentWindow { change_focused_to } => {
                self.close_current_window(change_focused_to)
            }
            Dispatch::OpenSearchPrompt {
                mode,
                scope,
                owner_id,
            } => self.open_search_prompt(mode, scope, owner_id),
            Dispatch::OpenFile { path } => {
                self.open_file(&path, true)?;
            }

            Dispatch::OpenFilePicker(kind) => {
                self.open_file_picker(kind)?;
            }
            Dispatch::RequestCompletion(params) => {
                self.lsp_manager.request_completion(params)?;
            }
            Dispatch::RequestReferences {
                params,
                include_declaration,
            } => self
                .lsp_manager
                .request_references(params, include_declaration)?,
            Dispatch::RequestHover(params) => {
                self.lsp_manager.request_hover(params)?;
            }
            Dispatch::RequestDefinitions(params) => {
                self.lsp_manager.request_definition(params)?;
            }
            Dispatch::RequestDeclarations(params) => {
                self.lsp_manager.request_declaration(params)?;
            }
            Dispatch::RequestImplementations(params) => {
                self.lsp_manager.request_implementation(params)?;
            }
            Dispatch::RequestTypeDefinitions(params) => {
                self.lsp_manager.request_type_definition(params)?;
            }
            Dispatch::PrepareRename(params) => {
                self.lsp_manager.prepare_rename_symbol(params)?;
            }
            Dispatch::RenameSymbol { params, new_name } => {
                self.lsp_manager.rename_symbol(params, new_name)?;
            }
            Dispatch::RequestCodeAction {
                params,
                diagnostics,
            } => {
                self.lsp_manager.request_code_action(params, diagnostics)?;
            }
            Dispatch::RequestSignatureHelp(params) => {
                self.lsp_manager.request_signature_help(params)?;
            }
            Dispatch::RequestDocumentSymbols(params) => {
                self.lsp_manager.request_document_symbols(params)?;
            }
            Dispatch::DocumentDidChange {
                path,
                content,
                language,
                component_id,
            } => {
                if let Some(language) = language {
                    self.request_syntax_highlight(component_id, language, content.clone())?;
                    // let highlight_spans = self.context.highlight(language, &content)?;
                    // self.update_highlighted_spans(component_id, highlight_spans)?
                }
                if let Some(path) = path {
                    self.lsp_manager.document_did_change(path, content)?;
                }
            }
            Dispatch::DocumentDidSave { path } => {
                self.lsp_manager.document_did_save(path)?;
            }
            Dispatch::ShowInfo { title, info } => self.show_info(&title, info),
            Dispatch::SetQuickfixList(r#type) => self.set_quickfix_list_type(r#type)?,
            Dispatch::GotoQuickfixListItem(direction) => self.goto_quickfix_list_item(direction)?,
            Dispatch::GotoSelectionHistoryFile(movement) => {
                self.goto_selection_history_file(movement)?;
                self.show_selection_history()
            }
            Dispatch::GotoSelectionHistoryContiguous(movement) => {
                self.goto_selection_history_contiguous(movement)?;
                self.show_selection_history()
            }
            Dispatch::ApplyWorkspaceEdit(workspace_edit) => {
                self.apply_workspace_edit(workspace_edit)?;
            }
            Dispatch::ShowKeymapLegend(keymap_legend_config) => {
                self.show_keymap_legend(keymap_legend_config)
            }

            #[cfg(test)]
            Dispatch::Custom(_) => unreachable!(),
            Dispatch::CloseAllExceptMainPanel => self.layout.close_all_except_main_panel(),
            Dispatch::DispatchEditor(dispatch_editor) => {
                self.handle_dispatch_editor(dispatch_editor)?
            }
            Dispatch::GotoLocation(location) => self.go_to_location(&location)?,
            Dispatch::GlobalSearch => self.global_search()?,
            Dispatch::OpenMoveToIndexPrompt => self.open_move_to_index_prompt(),
            Dispatch::RunCommand(command) => self.run_command(command)?,
            Dispatch::QuitAll => self.quit_all()?,
            Dispatch::OpenCommandPrompt => self.open_command_prompt(),
            Dispatch::SaveQuitAll => self.save_quit_all()?,
            Dispatch::RevealInExplorer(path) => self.layout.reveal_path_in_explorer(&path)?,
            Dispatch::OpenYesNoPrompt(prompt) => self.open_yes_no_prompt(prompt)?,
            Dispatch::OpenMoveFilePrompt(path) => self.open_move_file_prompt(path),
            Dispatch::OpenAddPathPrompt(path) => self.open_add_path_prompt(path),
            Dispatch::DeletePath(path) => self.delete_path(&path)?,
            Dispatch::Null => {
                // do nothing
            }
            Dispatch::MoveFile { from, to } => self.move_file(from, to)?,
            Dispatch::AddPath(path) => self.add_path(path)?,
            Dispatch::RefreshFileExplorer => {
                self.layout.refresh_file_explorer(&self.working_directory)?
            }
            Dispatch::SetClipboardContent(content) => self.context.set_clipboard_content(content),
            Dispatch::SetGlobalMode(mode) => self.set_global_mode(mode),

            Dispatch::HandleKeyEvent(key_event) => {
                self.handle_event(Event::Key(key_event))?;
            }
            Dispatch::GetRepoGitHunks => self.get_repo_git_hunks()?,
            Dispatch::SaveAll => self.save_all()?,
            Dispatch::TerminalDimensionChanged(dimension) => self.resize(dimension),
            Dispatch::SetGlobalTitle(title) => self.set_global_title(title),
            Dispatch::OpenInsideOtherPromptOpen => self.open_inside_other_prompt_open(),
            Dispatch::OpenInsideOtherPromptClose { open } => {
                self.open_inside_other_prompt_close(open)
            }
            Dispatch::OpenOmitLiteralPrompt { kind, target } => self.open_omit_prompt(
                kind,
                target,
                "Literal",
                Box::new(|text| Ok(FilterMechanism::Literal(text.to_string()))),
            ),
            Dispatch::OpenOmitRegexPrompt { kind, target } => self.open_omit_prompt(
                kind,
                target,
                "Regex",
                Box::new(|text| Ok(FilterMechanism::Regex(regex::Regex::new(text)?))),
            ),
            Dispatch::LspExecuteCommand { command, params } => self
                .lsp_manager
                .workspace_execute_command(params, command)?,
            Dispatch::PushSelectionSet {
                new_selection_set,
                old_selection_set,
                path,
            } => self.push_selection_set(path, old_selection_set, new_selection_set)?,
            Dispatch::UpdateLocalSearchConfig {
                update,
                owner_id,
                scope,
                show_legend,
            } => self.update_local_search_config(update, owner_id, scope, show_legend)?,
            Dispatch::UpdateGlobalSearchConfig { owner_id, update } => {
                self.update_global_search_config(owner_id, update)?;
            }
            Dispatch::OpenSetGlobalSearchFilterGlobPrompt {
                owner_id,
                filter_glob,
            } => self.open_set_global_search_filter_glob_prompt(owner_id, filter_glob),
            Dispatch::ShowSearchConfig { owner_id, scope } => {
                self.show_search_config(owner_id, scope)
            }
            Dispatch::OpenUpdateReplacementPrompt { owner_id, scope } => {
                self.open_update_replacement_prompt(owner_id, scope)
            }
            Dispatch::OpenUpdateSearchPrompt { owner_id, scope } => {
                self.open_update_search_prompt(owner_id, scope)
            }
            Dispatch::Replace { scope } => match scope {
                Scope::Local => self.handle_dispatch_editor(DispatchEditor::Replace {
                    config: self.context.local_search_config().clone(),
                })?,
                Scope::Global => self.global_replace()?,
            },
        }
        Ok(())
    }

    pub fn current_component(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.current_component()
    }

    fn close_current_window(&mut self, change_focused_to: Option<ComponentId>) {
        self.layout.close_current_window(change_focused_to)
    }

    fn local_search(&mut self, owner_id: Option<ComponentId>) -> anyhow::Result<()> {
        let config = self.context.local_search_config();
        let search = config.search();
        if !search.is_empty() {
            self.handle_dispatch_editor_custom(
                DispatchEditor::SetSelectionMode(SelectionMode::Find {
                    search: Search {
                        mode: config.mode,
                        search,
                    },
                }),
                owner_id
                    .and_then(|owner_id| self.get_component_by_id(owner_id))
                    .or_else(|| self.current_component()),
            )?;
        }

        Ok(())
    }

    fn resize(&mut self, dimension: Dimension) {
        self.layout
            .set_terminal_dimension(dimension.decrement_height(GLOBAL_TITLE_BAR_HEIGHT));
    }

    fn open_move_to_index_prompt(&mut self) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Move to index".to_string(),
            history: vec![],
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                let index = text.parse::<usize>()?.saturating_sub(1);
                Ok([Dispatch::DispatchEditor(DispatchEditor::MoveSelection(
                    Movement::Index(index),
                ))]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: vec![],
            enter_selects_first_matching_item: false,
        });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_rename_prompt(&mut self, params: RequestParams, current_name: Option<String>) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Rename".to_string(),
            history: current_name.into_iter().collect_vec(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok(vec![Dispatch::RenameSymbol {
                    params: params.clone(),
                    new_name: text.to_string(),
                }])
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: vec![],
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_search_prompt(
        &mut self,
        mode: LocalSearchConfigMode,
        scope: Scope,
        owner_id: ComponentId,
    ) {
        let current_component = self.current_component().clone();
        let config = self.context.get_local_search_config(scope);
        log::info!("config.searches = {:#?}", config.searches());
        let prompt = Prompt::new(PromptConfig {
            title: format!("{:?} search ({})", scope, mode.display()),
            history: config.searches(),
            owner: current_component.clone(),
            items: self.words(),
            on_text_change: Box::new(|_, _| Ok(vec![])),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::UpdateLocalSearchConfig {
                    owner_id,
                    update: LocalSearchConfigUpdate::SetSearch(text.to_string()),
                    scope,
                    show_legend: false,
                }]
                .to_vec())
            }),
            enter_selects_first_matching_item: false,
        });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_inside_other_prompt_open(&mut self) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Inside (other): Open".to_string(),
            history: Vec::new(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::OpenInsideOtherPromptClose {
                    open: text.to_owned(),
                }]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_inside_other_prompt_close(&mut self, open: String) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: format!("Inside (other, open = '{}'): Close", open),
            history: Vec::new(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::DispatchEditor(DispatchEditor::EnterInsideMode(
                    InsideKind::Other {
                        open: open.clone(),
                        close: text.to_owned(),
                    },
                ))]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_add_path_prompt(&mut self, path: CanonicalizedPath) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Add path".to_string(),
            history: [path.display_absolute()].to_vec(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| Ok([Dispatch::AddPath(text.into())].to_vec())),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_move_file_prompt(&mut self, path: CanonicalizedPath) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Move file".to_string(),
            history: [path.display_absolute()].to_vec(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::MoveFile {
                    from: path.clone(),
                    to: text.try_into()?,
                }]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_symbol_picker(
        &mut self,
        component_id: ComponentId,
        symbols: Symbols,
    ) -> anyhow::Result<()> {
        let current_component = self.current_component();
        if !current_component
            .as_ref()
            .is_some_and(|component| component.borrow().id() == component_id)
        {
            return Ok(());
        }
        let prompt =
            Prompt::new(PromptConfig {
                title: "Symbols".to_string(),
                history: vec![],
                owner: current_component,
                on_text_change: Box::new(|_, _| Ok(vec![])),
                items: symbols
                    .symbols
                    .iter()
                    .map(|symbol| {
                        CompletionItem::from_label(format!("{} ({:?})", symbol.name, symbol.kind))
                    })
                    .collect_vec(),
                on_enter: Box::new(move |current_item, _| {
                    // TODO: make Prompt generic over the item type,
                    // so that we don't have to do this,
                    // i.e. we can just return the symbol directly,
                    // instead of having to find it again.
                    if let Some(symbol) = symbols.symbols.iter().find(|symbol| {
                        current_item == &format!("{} ({:?})", symbol.name, symbol.kind)
                    }) {
                        Ok(vec![Dispatch::GotoLocation(symbol.location.clone())])
                    } else {
                        Ok(vec![])
                    }
                }),
                enter_selects_first_matching_item: true,
            });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
        Ok(())
    }

    fn open_command_prompt(&mut self) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Command".to_string(),
            history: vec![],
            owner: current_component,
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::RunCommand(text.to_string())]
                    .into_iter()
                    .collect())
            }),
            on_text_change: Box::new(|_, _| Ok(vec![])),
            items: crate::command::commands()
                .iter()
                .flat_map(|command| command.to_completion_items())
                .collect(),
            enter_selects_first_matching_item: true,
        });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_file_picker(&mut self, kind: FilePickerKind) -> anyhow::Result<()> {
        let current_component = self.current_component().clone();

        let working_directory = self.working_directory.clone();
        let prompt = Prompt::new(PromptConfig {
            title: format!("Open file: {}", kind.display()),
            history: vec![],
            owner: current_component,
            on_enter: Box::new(move |current_item, _| {
                let path = working_directory.join(current_item)?;
                Ok(vec![Dispatch::OpenFile { path }])
            }),
            on_text_change: Box::new(|_, _| Ok(vec![])),
            items: {
                match kind {
                    FilePickerKind::NonGitIgnored => {
                        git::GitRepo::try_from(&self.working_directory)?.non_git_ignored_files()?
                    }
                    FilePickerKind::GitStatus => {
                        git::GitRepo::try_from(&self.working_directory)?.git_status_files()?
                    }
                    FilePickerKind::Opened => self.layout.get_opened_files(),
                }
                .into_iter()
                .filter_map(|path| path.display_relative_to(&self.working_directory).ok())
                .map(CompletionItem::from_label)
                .collect_vec()
            },
            enter_selects_first_matching_item: true,
        });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
        Ok(())
    }

    pub fn open_file(
        &mut self,
        path: &CanonicalizedPath,
        focus_editor: bool,
    ) -> anyhow::Result<Rc<RefCell<dyn Component>>> {
        // Check if the file is opened before
        // so that we won't notify the LSP twice
        if let Some(matching_editor) = self.layout.open_file(path) {
            return Ok(matching_editor);
        }

        let buffer = Buffer::from_path(path)?;
        let language = buffer.language();
        let content = buffer.content();
        let buffer = Rc::new(RefCell::new(buffer));
        let editor = SuggestiveEditor::from_buffer(buffer, SuggestiveEditorFilter::CurrentWord);
        let component_id = editor.id();
        let component = Rc::new(RefCell::new(editor));

        if focus_editor {
            self.push_selection_set(
                path.clone(),
                SelectionSet::default(),
                SelectionSet::default(),
            )?;
            self.layout
                .add_and_focus_suggestive_editor(component.clone());
        } else {
            self.layout.add_suggestive_editor(component.clone());
        }

        if let Some(language) = language {
            self.request_syntax_highlight(component_id, language, content)?;
        }

        if self.enable_lsp {
            self.lsp_manager.open_file(path.clone())?;
        }
        Ok(component)
    }

    fn get_suggestive_editor(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<Rc<RefCell<SuggestiveEditor>>> {
        self.layout.get_suggestive_editor(component_id)
    }

    pub fn handle_lsp_notification(&mut self, notification: LspNotification) -> anyhow::Result<()> {
        match notification {
            LspNotification::Hover(context, hover) => {
                self.get_suggestive_editor(context.component_id)?
                    .borrow_mut()
                    .show_info("Hover info", Info::new(hover.contents.join("\n\n")));
                Ok(())
            }
            LspNotification::Definition(context, response) => {
                match response {
                    GotoDefinitionResponse::Single(location) => self.go_to_location(&location)?,
                    GotoDefinitionResponse::Multiple(locations) => {
                        if locations.is_empty() {
                            self.show_info(
                                "Goto definition info",
                                Info::new("No definitions found".to_string()),
                            );
                        } else {
                            self.set_quickfix_list(
                                context,
                                QuickfixList::new(
                                    locations.into_iter().map(QuickfixListItem::from).collect(),
                                ),
                            )?
                        }
                    }
                }

                Ok(())
            }
            LspNotification::References(context, locations) => self.set_quickfix_list(
                context,
                QuickfixList::new(locations.into_iter().map(QuickfixListItem::from).collect()),
            ),
            LspNotification::Completion(context, completion) => {
                self.get_suggestive_editor(context.component_id)?
                    .borrow_mut()
                    .set_completion(completion);
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
                let diagnostics = params
                    .diagnostics
                    .into_iter()
                    .map(Diagnostic::try_from)
                    .collect::<Result<Vec<_>, _>>()?;
                self.update_diagnostics(
                    params
                        .uri
                        .to_file_path()
                        .map_err(|err| {
                            anyhow::anyhow!("Couldn't convert URI to file path: {:?}", err)
                        })?
                        .try_into()?,
                    diagnostics,
                );
                Ok(())
            }
            LspNotification::PrepareRenameResponse(context, response) => {
                let editor = self.get_suggestive_editor(context.component_id)?;
                log::info!("response = {:#?}", response);

                // Note: we cannot refactor the following code into the below code, otherwise we will get error,
                // because RefCell is borrow_mut twice. The borrow has to be dropped.
                //
                //
                //     if let Some(params) = editor.borrow().editor().get_request_params() {
                //         self.open_rename_prompt(params);
                //     }
                //
                let (params, current_name) = {
                    let editor = editor.borrow();
                    let params = editor.editor().get_request_params();
                    let buffer = editor.editor().buffer();
                    let current_name = response
                        .range
                        .map(|range| {
                            let range = buffer.position_to_char(range.start)?
                                ..buffer.position_to_char(range.end)?;
                            buffer.slice(&range.try_into()?)
                        })
                        .transpose()
                        .unwrap_or_default()
                        .map(|rope| rope.to_string());
                    (params, current_name)
                };
                if let Some(params) = params {
                    self.open_rename_prompt(params, current_name);
                }

                Ok(())
            }
            LspNotification::Error(error) => {
                self.show_info("LSP error", Info::new(error));
                Ok(())
            }
            LspNotification::WorkspaceEdit(workspace_edit) => {
                self.apply_workspace_edit(workspace_edit)
            }
            LspNotification::CodeAction(context, code_actions) => {
                let editor = self.get_suggestive_editor(context.component_id)?;
                editor.borrow_mut().set_code_actions(code_actions);
                Ok(())
            }
            LspNotification::SignatureHelp(context, signature_help) => {
                let editor = self.get_suggestive_editor(context.component_id)?;
                editor.borrow_mut().show_signature_help(signature_help);
                Ok(())
            }
            LspNotification::Symbols(context, symbols) => {
                self.open_symbol_picker(context.component_id, symbols)
            }
        }
    }

    fn update_diagnostics(&mut self, path: CanonicalizedPath, diagnostics: Vec<Diagnostic>) {
        self.context.update_diagnostics(path, diagnostics);
    }

    fn goto_quickfix_list_item(&mut self, movement: Movement) -> anyhow::Result<()> {
        let item = self
            .context
            .quickfix_lists()
            .borrow_mut()
            .get_item(movement);
        if let Some(item) = item {
            self.go_to_location(item.location())?;
        }
        Ok(())
    }

    fn show_info(&mut self, title: &str, info: Info) {
        self.layout.show_info(title, info).unwrap_or_else(|err| {
            log::error!("Error showing info: {:?}", err);
        });
    }

    fn go_to_location(&mut self, location: &Location) -> Result<(), anyhow::Error> {
        let component = self.open_file(&location.path, true)?;
        component
            .borrow_mut()
            .editor_mut()
            .set_selection(location.range.clone())?;
        Ok(())
    }

    fn set_quickfix_list_type(&mut self, r#type: QuickfixListType) -> anyhow::Result<()> {
        match r#type {
            QuickfixListType::LspDiagnostic(severity) => {
                let quickfix_list = QuickfixList::new(
                    self.context
                        .diagnostics()
                        .into_iter()
                        .filter(|(_, diagnostic)| diagnostic.severity == severity)
                        .map(|(path, diagnostic)| {
                            QuickfixListItem::new(
                                Location {
                                    path: (*path).clone(),
                                    range: diagnostic.range.clone(),
                                },
                                Some(Info::new(diagnostic.message())),
                            )
                        })
                        .collect(),
                );

                self.set_quickfix_list(
                    ResponseContext::default().set_description("Diagnostic"),
                    quickfix_list,
                )
            }
            QuickfixListType::Items(items) => {
                self.set_quickfix_list(ResponseContext::default(), QuickfixList::new(items))
            }
            QuickfixListType::Bookmark => {
                let quickfix_list = QuickfixList::new(
                    self.layout
                        .buffers()
                        .into_iter()
                        .flat_map(|buffer| {
                            buffer
                                .borrow()
                                .bookmarks()
                                .into_iter()
                                .filter_map(|bookmark| {
                                    let buffer = buffer.borrow();
                                    let position_range =
                                        buffer.char_index_range_to_position_range(bookmark).ok()?;
                                    Some(QuickfixListItem::new(
                                        Location {
                                            path: buffer.path()?,
                                            range: position_range,
                                        },
                                        None,
                                    ))
                                })
                                .collect_vec()
                        })
                        .collect_vec(),
                );
                self.set_quickfix_list(
                    ResponseContext::default().set_description("Bookmark"),
                    quickfix_list,
                )
            }
        }
    }

    fn set_quickfix_list(
        &mut self,
        context: ResponseContext,
        quickfix_list: QuickfixList,
    ) -> anyhow::Result<()> {
        self.context.set_mode(Some(GlobalMode::QuickfixListItem));
        self.context
            .quickfix_lists()
            .borrow_mut()
            .push(quickfix_list.set_title(context.description.clone()));
        match context.scope {
            None | Some(Scope::Global) => {
                self.layout
                    .show_quickfix_lists(self.context.quickfix_lists().clone());
                self.goto_quickfix_list_item(Movement::Next)
            }
            Some(Scope::Local) => self.handle_dispatch(Dispatch::DispatchEditor(
                DispatchEditor::SetSelectionMode(SelectionMode::LocalQuickfix {
                    title: context.description.unwrap_or_default(),
                }),
            )),
        }
    }

    fn apply_workspace_edit(&mut self, workspace_edit: WorkspaceEdit) -> Result<(), anyhow::Error> {
        // TODO: should we wrap this in a transaction so that if one of the edit/operation fails, the whole transaction fails?
        // Such that it won't leave the workspace in an half-edited messed up state
        for edit in workspace_edit.edits {
            let component = self.open_file(&edit.path, false)?;
            let dispatches = component
                .borrow_mut()
                .editor_mut()
                .apply_positional_edits(edit.edits)?;

            self.handle_dispatches(dispatches)?;

            let dispatches = component.borrow_mut().editor_mut().save()?;

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
        self.layout.show_keymap_legend(keymap_legend_config)
    }

    fn global_replace(&mut self) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        let global_search_config = self.context.global_search_config();
        let walk_builder_config = WalkBuilderConfig {
            root: working_directory.clone().into(),
            include: global_search_config.include_glob(),
            exclude: global_search_config.exclude_glob(),
        };
        let config = self.context.global_search_config().local_config.clone();
        match config.mode {
            LocalSearchConfigMode::Regex(_) => {
                let affected_paths = list::grep::replace(walk_builder_config, config)?;
                self.layout.reload_buffers(affected_paths)?;

                Ok(())
            }
            LocalSearchConfigMode::AstGrep => {
                todo!()
            }
        }
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
        let locations = match config.mode {
            LocalSearchConfigMode::Regex(regex) => {
                list::grep::run(&config.search(), walk_builder_config, regex)
            }
            LocalSearchConfigMode::AstGrep => {
                list::ast_grep::run(config.search().clone(), walk_builder_config)
            }
        }?;
        self.set_quickfix_list(
            ResponseContext::default().set_description("Global search"),
            QuickfixList::new(
                locations
                    .into_iter()
                    .map(|location| QuickfixListItem::new(location, None))
                    .collect_vec(),
            ),
        )
    }

    pub fn quit_all(&self) -> Result<(), anyhow::Error> {
        Ok(self.sender.send(AppMessage::QuitAll)?)
    }

    pub fn sender(&self) -> Sender<AppMessage> {
        self.sender.clone()
    }

    fn run_command(&mut self, command: String) -> anyhow::Result<()> {
        let dispatch = crate::command::find(&command)
            .map(|cmd| cmd.dispatch())
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", command))?;
        self.handle_dispatch(dispatch)
    }

    fn save_quit_all(&mut self) -> anyhow::Result<()> {
        self.save_all()?;
        self.quit_all()?;
        Ok(())
    }

    fn save_all(&self) -> anyhow::Result<()> {
        self.layout.save_all()
    }

    fn open_yes_no_prompt(&mut self, prompt: YesNoPrompt) -> anyhow::Result<()> {
        self.handle_dispatch(Dispatch::ShowKeymapLegend(KeymapLegendConfig {
            title: "Prompt".to_string(),
            owner_id: prompt.owner_id,
            body: KeymapLegendBody::MultipleSections {
                sections: [KeymapLegendSection {
                    title: prompt.title,
                    keymaps: Keymaps::new(&[
                        Keymap::new("y", "Yes".to_string(), *prompt.yes),
                        Keymap::new("n", "No".to_string(), Dispatch::Null),
                    ]),
                }]
                .to_vec(),
            },
        }))
    }

    fn delete_path(&mut self, path: &CanonicalizedPath) -> anyhow::Result<()> {
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        self.layout.remove_suggestive_editor(path);
        self.layout.refresh_file_explorer(&self.working_directory)?;
        Ok(())
    }

    fn move_file(&mut self, from: CanonicalizedPath, to: PathBuf) -> anyhow::Result<()> {
        use std::fs;
        log::info!(
            "move file from {} to {}",
            from.display_absolute(),
            to.display()
        );
        self.add_path_parent(&to)?;
        fs::rename(from.clone(), to.clone())?;
        self.layout.refresh_file_explorer(&self.working_directory)?;
        let to = to.try_into()?;
        self.layout.reveal_path_in_explorer(&to)?;
        self.lsp_manager.document_did_rename(from, to)?;
        Ok(())
    }
    fn add_path_parent(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(new_dir) = path.parent() {
            log::info!("Creating new dir at {}", new_dir.display());
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
        self.layout.refresh_file_explorer(&self.working_directory)?;
        self.layout.reveal_path_in_explorer(&path.try_into()?)?;

        Ok(())
    }

    fn request_syntax_highlight(
        &self,
        component_id: ComponentId,
        language: Language,
        content: String,
    ) -> anyhow::Result<()> {
        if let Some(sender) = &self.syntax_highlight_request_sender {
            sender.send(SyntaxHighlightRequest {
                component_id,
                language,
                source_code: content,
                theme: Box::new(VSCODE_LIGHT),
            })?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn get_selected_texts(&mut self, path: &CanonicalizedPath) -> Vec<String> {
        self.layout
            .open_file(path)
            .unwrap()
            .borrow()
            .editor()
            .get_selected_texts()
    }

    #[cfg(test)]
    pub fn get_file_content(&mut self, path: &CanonicalizedPath) -> String {
        self.layout.open_file(path).unwrap().borrow().content()
    }

    #[cfg(test)]
    pub fn handle_dispatch_editors(
        &mut self,
        dispatch_editors: &[DispatchEditor],
    ) -> anyhow::Result<()> {
        for dispatch_editor in dispatch_editors {
            self.handle_dispatch_editor(dispatch_editor.clone())?;
        }
        Ok(())
    }

    fn handle_dispatch_editor(&mut self, dispatch_editor: DispatchEditor) -> anyhow::Result<()> {
        self.handle_dispatch_editor_custom(dispatch_editor, self.current_component())
    }

    fn handle_dispatch_editor_custom(
        &mut self,
        dispatch_editor: DispatchEditor,
        component: Option<Rc<RefCell<dyn Component>>>,
    ) -> anyhow::Result<()> {
        if let Some(component) = component {
            let dispatches = component
                .borrow_mut()
                .editor_mut()
                .apply_dispatch(&mut self.context, dispatch_editor)?;

            self.handle_dispatches(dispatches)?;
        }
        Ok(())
    }

    fn get_repo_git_hunks(&mut self) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        let repo = git::GitRepo::try_from(&working_directory)?;
        let diffs = repo.diffs()?;
        self.set_quickfix_list(
            ResponseContext::default().set_description("Repo Git Hunks"),
            QuickfixList::new(
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

    pub(crate) fn get_quickfixes(&self) -> Vec<QuickfixListItem> {
        self.layout.get_quickfixes().unwrap_or_default()
    }

    fn set_global_title(&mut self, title: String) {
        self.global_title = Some(title)
    }

    pub fn set_syntax_highlight_request_sender(&mut self, sender: Sender<SyntaxHighlightRequest>) {
        self.syntax_highlight_request_sender = Some(sender);
    }

    pub(crate) fn get_current_file_path(&self) -> Option<CanonicalizedPath> {
        self.current_component()
            .and_then(|component| component.borrow().path())
    }

    pub(crate) fn get_current_info(&self) -> Option<String> {
        self.layout.get_info()
    }

    fn set_global_mode(&mut self, mode: Option<GlobalMode>) {
        if mode == Some(GlobalMode::SelectionHistoryFile) {
            self.show_selection_history()
        }
        self.context.set_mode(mode);
    }

    pub fn display_selection_history(&self) -> String {
        self.undo_tree.display().to_string()
    }

    fn show_selection_history(&mut self) {
        let tree = self.display_selection_history();
        self.show_info("Selection History", Info::new(tree));
    }

    fn open_omit_prompt(
        &mut self,
        kind: FilterKind,
        target: FilterTarget,
        mechanism: &str,
        make_filter_mechanism: Box<dyn Fn(&str) -> anyhow::Result<FilterMechanism>>,
    ) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: format!(
                "Omit: {:?} selection by {:?} matching {}",
                kind, target, mechanism
            ),
            history: Vec::new(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::DispatchEditor(DispatchEditor::FilterPush(
                    Filter::new(kind, target, make_filter_mechanism(text)?),
                ))]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn push_selection_set(
        &mut self,
        path: CanonicalizedPath,
        old_selection_set: SelectionSet,
        new_selection_set: SelectionSet,
    ) -> anyhow::Result<()> {
        let new_to_old = self
            .layout
            .current_component()
            .and_then(|component| {
                let current_path = self.get_current_file_path()?;
                if current_path != path {
                    Some(FileSelectionSet {
                        path: current_path,
                        selection_set: component.borrow().editor().selection_set.clone(),
                    })
                } else {
                    None
                }
            })
            .unwrap_or_else(|| FileSelectionSet {
                path: path.clone(),
                selection_set: old_selection_set.clone(),
            });
        self.undo_tree.edit(
            &mut self.layout,
            crate::undo_tree::OldNew {
                old_to_new: FileSelectionSet {
                    path: path.clone(),
                    selection_set: new_selection_set,
                },
                new_to_old,
            },
        )?;
        Ok(())
    }

    pub(crate) fn get_current_selection_set(&self) -> Option<SelectionSet> {
        Some(
            self.current_component()?
                .borrow()
                .editor()
                .selection_set
                .clone(),
        )
    }

    fn goto_selection_history_contiguous(&mut self, movement: Movement) -> anyhow::Result<()> {
        match movement {
            Movement::Next => {
                self.undo_tree.redo(&mut self.layout)?;
            }
            Movement::Previous => {
                self.undo_tree.undo(&mut self.layout)?;
            }
            Movement::Up => {
                self.set_global_mode(Some(GlobalMode::SelectionHistoryFile));
            }
            _ => {}
        }
        Ok(())
    }

    fn goto_selection_history_file(&mut self, movement: Movement) -> anyhow::Result<()> {
        match movement {
            Movement::Next => {
                let next_entries = self
                    .undo_tree
                    .next_entries()
                    .into_iter()
                    .filter(|(_, entry)| {
                        Some(&entry.get().old_to_new.path) != self.get_current_file_path().as_ref()
                    })
                    .collect_vec();
                log::info!("next_entries.len() = {}", next_entries.len());
                log::info!(
                    "next_entries.last().index = {:?}",
                    next_entries.last().map(|e| e.0)
                );
                if let Some((next_entry_index, next_entry)) = next_entries.first() {
                    let next_file = next_entry.get().old_to_new.path.clone();
                    let next_entry_index = *next_entry_index;
                    let index = next_entries
                        .into_iter()
                        .take_while(|(_, entry)| entry.get().old_to_new.path == next_file)
                        .last()
                        .map(|(index, _)| index)
                        .unwrap_or(next_entry_index);
                    self.undo_tree.go_to_entry(&mut self.layout, index)?;
                }
            }
            Movement::Previous => {
                if let Some((index, entry)) =
                    self.undo_tree
                        .previous_entries()
                        .into_iter()
                        .rfind(|(_, entry)| {
                            Some(&entry.get().old_to_new.path)
                                != self.get_current_file_path().as_ref()
                        })
                {
                    log::info!("entry.path = {:?}", entry.get().new_to_old.path);
                    self.undo_tree.go_to_entry(&mut self.layout, index)?;
                }
            }
            Movement::Down => {
                self.set_global_mode(Some(GlobalMode::SelectionHistoryContiguous));
            }
            _ => {}
        };
        Ok(())
    }

    pub(crate) fn context(&self) -> &Context {
        &self.context
    }

    fn update_local_search_config(
        &mut self,
        update: LocalSearchConfigUpdate,
        owner_id: ComponentId,
        scope: Scope,
        show_legend: bool,
    ) -> Result<(), anyhow::Error> {
        self.context.update_local_search_config(update, scope);
        match scope {
            Scope::Local => self.local_search(Some(owner_id))?,
            Scope::Global => {
                self.global_search()?;
            }
        }

        if show_legend {
            self.show_search_config(owner_id, scope);
        }
        Ok(())
    }

    fn update_global_search_config(
        &mut self,
        owner_id: ComponentId,
        update: GlobalSearchConfigUpdate,
    ) -> anyhow::Result<()> {
        self.context.update_global_search_config(update)?;
        self.global_search()?;
        self.show_search_config(owner_id, Scope::Global);
        Ok(())
    }

    fn get_component_by_id(&self, id: ComponentId) -> Option<Rc<RefCell<dyn Component>>> {
        self.components()
            .into_iter()
            .find(|component| component.borrow().id() == id)
    }

    fn open_set_global_search_filter_glob_prompt(
        &mut self,
        owner_id: ComponentId,
        filter_glob: GlobalSearchFilterGlob,
    ) {
        let current_component = self.current_component().clone();
        let config = self.context.global_search_config();
        let initial_text = match filter_glob {
            GlobalSearchFilterGlob::Include => config.include_glob(),
            GlobalSearchFilterGlob::Exclude => config.exclude_glob(),
        }
        .map(|glob| glob.to_string());
        let prompt = Prompt::new(PromptConfig {
            title: format!("Set global search {:?} files glob", filter_glob),
            history: initial_text.into_iter().collect_vec(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::UpdateGlobalSearchConfig {
                    owner_id,
                    update: GlobalSearchConfigUpdate::SetGlob(filter_glob, text.to_string()),
                }]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn show_search_config(&mut self, owner_id: ComponentId, scope: Scope) {
        fn show_checkbox(title: &str, checked: bool) -> String {
            format!("{title} [{}]", if checked { "X" } else { " " })
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
                        owner_id,
                        scope,
                        show_legend: true,
                    },
                )
            };
        let update_mode_keymap = |key: &'static str,
                                  name: String,
                                  mode: LocalSearchConfigMode,
                                  checked: bool|
         -> Keymap {
            let description = show_checkbox(&name, checked);
            update_keymap(key, description, LocalSearchConfigUpdate::SetMode(mode))
        };
        let regex = match local_search_config.mode {
            LocalSearchConfigMode::Regex(regex) => Some(regex),
            LocalSearchConfigMode::AstGrep => None,
        };
        self.show_keymap_legend(KeymapLegendConfig {
            title: format!("Configure Search ({:?})", scope),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    KeymapLegendSection {
                        title: "Inputs".to_string(),
                        keymaps: Keymaps::new(
                            &[
                                Keymap::new(
                                    "s",
                                    format!("Search = {}", local_search_config.search()),
                                    Dispatch::OpenUpdateSearchPrompt { owner_id, scope },
                                ),
                                Keymap::new(
                                    "r",
                                    format!("Replacement = {}", local_search_config.replacement()),
                                    Dispatch::OpenUpdateReplacementPrompt { owner_id, scope },
                                ),
                            ]
                            .into_iter()
                            .chain(
                                global_search_confing
                                    .map(|config| {
                                        [
                                            Keymap::new(
                                                "i",
                                                format!(
                                                    "Include files (glob) = {}",
                                                    config
                                                        .include_glob()
                                                        .map(|glob| glob.to_string())
                                                        .unwrap_or_default()
                                                ),
                                                Dispatch::OpenSetGlobalSearchFilterGlobPrompt {
                                                    owner_id,
                                                    filter_glob: GlobalSearchFilterGlob::Include,
                                                },
                                            ),
                                            Keymap::new(
                                                "e",
                                                format!(
                                                    "Exclude files (glob) = {}",
                                                    config
                                                        .exclude_glob()
                                                        .map(|glob| glob.to_string())
                                                        .unwrap_or_default()
                                                ),
                                                Dispatch::OpenSetGlobalSearchFilterGlobPrompt {
                                                    owner_id,
                                                    filter_glob: GlobalSearchFilterGlob::Exclude,
                                                },
                                            ),
                                        ]
                                        .to_vec()
                                    })
                                    .unwrap_or_default(),
                            )
                            .collect_vec(),
                        ),
                    },
                    KeymapLegendSection {
                        title: "Mode".to_string(),
                        keymaps: Keymaps::new(&[
                            update_mode_keymap(
                                "l",
                                "Literal".to_string(),
                                LocalSearchConfigMode::Regex(RegexConfig {
                                    escaped: true,
                                    ..regex.unwrap_or_default()
                                }),
                                regex.map(|regex| regex.escaped).unwrap_or(false),
                            ),
                            update_mode_keymap(
                                "x",
                                "Regex".to_string(),
                                LocalSearchConfigMode::Regex(RegexConfig {
                                    escaped: false,
                                    ..regex.unwrap_or_default()
                                }),
                                regex.map(|regex| !regex.escaped).unwrap_or(false),
                            ),
                            update_mode_keymap(
                                "a",
                                "AST Grep".to_string(),
                                LocalSearchConfigMode::AstGrep,
                                local_search_config.mode == LocalSearchConfigMode::AstGrep,
                            ),
                        ]),
                    },
                    KeymapLegendSection {
                        title: "Actions".to_string(),
                        keymaps: Keymaps::new(&[Keymap::new(
                            "R",
                            "Replace all".to_string(),
                            Dispatch::Replace { scope },
                        )]),
                    },
                ]
                .into_iter()
                .chain(regex.map(|regex| {
                    KeymapLegendSection {
                        title: "Options".to_string(),
                        keymaps: Keymaps::new(
                            &[
                                update_mode_keymap(
                                    "c",
                                    "Case-sensitive".to_string(),
                                    LocalSearchConfigMode::Regex(RegexConfig {
                                        case_sensitive: !regex.case_sensitive,
                                        ..regex
                                    }),
                                    regex.case_sensitive,
                                ),
                                update_mode_keymap(
                                    "w",
                                    "Match whole word".to_string(),
                                    LocalSearchConfigMode::Regex(RegexConfig {
                                        match_whole_word: !regex.match_whole_word,
                                        ..regex
                                    }),
                                    regex.match_whole_word,
                                ),
                            ]
                            .into_iter()
                            .collect_vec(),
                        ),
                    }
                }))
                .collect(),
            },
            owner_id,
        })
    }

    fn open_update_replacement_prompt(&mut self, owner_id: ComponentId, scope: Scope) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: format!("Set Replace ({:?})", scope),
            history: self.context.get_local_search_config(scope).replacements(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::UpdateLocalSearchConfig {
                    owner_id,
                    scope,
                    update: LocalSearchConfigUpdate::SetReplacement(text.to_owned()),
                    show_legend: true,
                }]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_update_search_prompt(&mut self, owner_id: ComponentId, scope: Scope) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: format!("Set Search ({:?})", scope),
            history: self.context.get_local_search_config(scope).replacements(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::UpdateLocalSearchConfig {
                    owner_id,
                    scope,
                    update: LocalSearchConfigUpdate::SetSearch(text.to_owned()),
                    show_legend: true,
                }]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: self.words(),
            enter_selects_first_matching_item: false,
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn words(&self) -> Vec<CompletionItem> {
        self.current_component()
            .map(|current_component| {
                current_component
                    .borrow()
                    .editor()
                    .buffer()
                    .words()
                    .into_iter()
                    .map(CompletionItem::from_label)
                    .collect_vec()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dimension {
    pub height: u16,
    pub width: u16,
}

impl Dimension {
    pub fn area(&self) -> usize {
        self.height as usize * self.width as usize
    }

    pub fn positions(&self) -> HashSet<Position> {
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

#[derive(Clone, Debug, PartialEq, Eq)]
/// Dispatch are for child component to request action from the root node
pub enum Dispatch {
    CloseCurrentWindow {
        change_focused_to: Option<ComponentId>,
    },
    OpenFilePicker(FilePickerKind),
    OpenSearchPrompt {
        mode: LocalSearchConfigMode,
        scope: Scope,
        owner_id: ComponentId,
    },
    OpenFile {
        path: CanonicalizedPath,
    },
    ShowInfo {
        title: String,
        info: Info,
    },
    RequestCompletion(RequestParams),
    RequestSignatureHelp(RequestParams),
    RequestHover(RequestParams),
    RequestDefinitions(RequestParams),
    RequestDeclarations(RequestParams),
    RequestImplementations(RequestParams),
    RequestTypeDefinitions(RequestParams),
    RequestReferences {
        params: RequestParams,
        include_declaration: bool,
    },
    PrepareRename(RequestParams),
    RequestCodeAction {
        params: RequestParams,
        diagnostics: Vec<lsp_types::Diagnostic>,
    },
    RenameSymbol {
        params: RequestParams,
        new_name: String,
    },
    DocumentDidChange {
        component_id: ComponentId,
        path: Option<CanonicalizedPath>,
        content: String,
        language: Option<Language>,
    },
    DocumentDidSave {
        path: CanonicalizedPath,
    },
    SetQuickfixList(QuickfixListType),
    GotoQuickfixListItem(Movement),
    GotoSelectionHistoryFile(Movement),
    ApplyWorkspaceEdit(WorkspaceEdit),
    ShowKeymapLegend(KeymapLegendConfig),
    CloseAllExceptMainPanel,

    #[cfg(test)]
    /// Used for testing
    Custom(String),
    DispatchEditor(DispatchEditor),
    RequestDocumentSymbols(RequestParams),
    GotoLocation(Location),
    GlobalSearch,
    OpenMoveToIndexPrompt,
    RunCommand(String),
    QuitAll,
    OpenCommandPrompt,
    SaveQuitAll,
    RevealInExplorer(CanonicalizedPath),
    OpenYesNoPrompt(YesNoPrompt),
    OpenMoveFilePrompt(CanonicalizedPath),
    OpenAddPathPrompt(CanonicalizedPath),
    DeletePath(CanonicalizedPath),
    Null,
    MoveFile {
        from: CanonicalizedPath,
        to: PathBuf,
    },
    AddPath(String),
    RefreshFileExplorer,
    SetClipboardContent(String),
    SetGlobalMode(Option<GlobalMode>),
    HandleKeyEvent(event::KeyEvent),
    GetRepoGitHunks,
    SaveAll,
    TerminalDimensionChanged(Dimension),
    SetGlobalTitle(String),
    OpenInsideOtherPromptOpen,
    OpenInsideOtherPromptClose {
        open: String,
    },
    OpenOmitLiteralPrompt {
        kind: FilterKind,
        target: FilterTarget,
    },
    OpenOmitRegexPrompt {
        kind: FilterKind,
        target: FilterTarget,
    },
    LspExecuteCommand {
        params: RequestParams,
        command: crate::lsp::code_action::Command,
    },
    PushSelectionSet {
        new_selection_set: SelectionSet,
        old_selection_set: SelectionSet,
        path: CanonicalizedPath,
    },
    GotoSelectionHistoryContiguous(Movement),
    UpdateLocalSearchConfig {
        owner_id: ComponentId,
        update: LocalSearchConfigUpdate,
        scope: Scope,
        show_legend: bool,
    },
    UpdateGlobalSearchConfig {
        owner_id: ComponentId,
        update: GlobalSearchConfigUpdate,
    },
    OpenSetGlobalSearchFilterGlobPrompt {
        owner_id: ComponentId,
        filter_glob: GlobalSearchFilterGlob,
    },
    ShowSearchConfig {
        owner_id: ComponentId,
        scope: Scope,
    },
    OpenUpdateReplacementPrompt {
        owner_id: ComponentId,
        scope: Scope,
    },
    OpenUpdateSearchPrompt {
        owner_id: ComponentId,
        scope: Scope,
    },
    Replace {
        scope: Scope,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlobalSearchConfigUpdate {
    SetGlob(GlobalSearchFilterGlob, String),
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum GlobalSearchFilterGlob {
    Include,
    Exclude,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalSearchConfigUpdate {
    SetMode(LocalSearchConfigMode),
    SetReplacement(String),
    SetSearch(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YesNoPrompt {
    pub title: String,
    pub owner_id: ComponentId,
    pub yes: Box<Dispatch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilePickerKind {
    NonGitIgnored,
    GitStatus,
    Opened,
}
impl FilePickerKind {
    pub fn display(&self) -> String {
        match self {
            FilePickerKind::NonGitIgnored => "Not Git Ignored".to_string(),
            FilePickerKind::GitStatus => "Git Status".to_string(),
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

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum Scope {
    Local,
    Global,
}

#[derive(Debug)]
pub enum AppMessage {
    LspNotification(LspNotification),
    Event(Event),
    QuitAll,
    SyntaxHighlightResponse {
        component_id: ComponentId,
        highlighted_spans: HighlighedSpans,
    },
}
