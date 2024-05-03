use crate::{
    buffer::Buffer,
    components::{
        component::{Component, ComponentId, GetGridResult},
        dropdown::{DropdownItem, DropdownRender},
        editor::{DispatchEditor, Editor, Movement},
        keymap_legend::{
            Keymap, KeymapLegendBody, KeymapLegendConfig, KeymapLegendSection, Keymaps,
        },
        prompt::{Prompt, PromptConfig},
        suggestive_editor::{
            DispatchSuggestiveEditor, Info, SuggestiveEditor, SuggestiveEditorFilter,
        },
    },
    context::{Context, GlobalMode, LocalSearchConfigMode, QuickfixListSource, Search},
    frontend::Frontend,
    git,
    grid::{Grid, LineUpdate},
    history::History,
    layout::Layout,
    list::{self, grep::RegexConfig, WalkBuilderConfig},
    lsp::{
        goto_definition_response::GotoDefinitionResponse,
        manager::LspManager,
        process::{LspNotification, ResponseContext},
        symbols::Symbols,
        workspace_edit::WorkspaceEdit,
    },
    position::Position,
    quickfix_list::{Location, QuickfixList, QuickfixListItem, QuickfixListType},
    screen::{Screen, Window},
    selection::{Filter, FilterKind, FilterMechanism, FilterTarget, SelectionMode, SelectionSet},
    selection_mode::inside::InsideKind,
    syntax_highlight::{HighlighedSpans, SyntaxHighlightRequest},
    themes::Theme,
    ui_tree::{ComponentKind, KindedComponent},
};
use event::event::Event;
use itertools::Itertools;
use shared::{canonicalized_path::CanonicalizedPath, language::Language};
use std::{
    cell::RefCell,
    collections::HashSet,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};
use DispatchEditor::*;

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

    selection_set_history: History<SelectionSetHistory>,
}

#[derive(PartialEq, Clone, Debug, Eq)]
struct SelectionSetHistory {
    kind: SelectionSetHistoryKind,
    selection_set: SelectionSet,
}

#[derive(PartialEq, Clone, Debug, Eq)]
pub enum SelectionSetHistoryKind {
    Path(CanonicalizedPath),
    ComponentId(Option<ComponentId>),
}

struct Null;
impl std::fmt::Display for Null {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NULL")
    }
}

const GLOBAL_TITLE_BAR_HEIGHT: u16 = 1;
impl<T: Frontend> App<T> {
    #[cfg(test)]
    pub fn new(
        frontend: Arc<Mutex<T>>,
        working_directory: CanonicalizedPath,
    ) -> anyhow::Result<App<T>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self::from_channel(frontend, working_directory, sender, receiver)
    }

    #[cfg(test)]
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

            selection_set_history: History::default(),
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
                    highlighted_spans,
                } => self
                    .update_highlighted_spans(component_id, highlighted_spans)
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

    pub fn quit(&mut self) -> anyhow::Result<()> {
        let mut frontend = self.frontend.lock().unwrap();
        frontend.leave_alternate_screen()?;
        frontend.disable_raw_mode()?;
        // self.lsp_manager.shutdown();

        std::process::exit(0);
    }

    pub fn components(&self) -> Vec<KindedComponent> {
        self.layout.components()
    }

    /// Returns true if the app should quit.
    fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        // Pass event to focused window
        let component = self.current_component();
        match event {
            // Event::Key(key!("enter")) if self.context.mode().is_some() => { self.context.set_mode(None); }
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

    fn render(&mut self) -> Result<(), anyhow::Error> {
        let screen = self.get_screen()?;
        self.render_screen(screen)?;
        Ok(())
    }

    pub fn get_screen(&mut self) -> Result<Screen, anyhow::Error> {
        // Recalculate layout before each render
        self.layout.recalculate_layout();

        // Generate layout
        let dimension = self.layout.terminal_dimension();
        // Render every window
        let (windows, cursors): (Vec<_>, Vec<_>) = self
            .components()
            .into_iter()
            .map(|component| {
                let rectangle = component.component().borrow().rectangle().clone();
                let GetGridResult { grid, cursor } =
                    component.component().borrow().get_grid(&self.context);
                let focused_component_id = self.layout.focused_component_id();
                let cursor_position = if component.component().borrow().id() == focused_component_id
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
                let window = Window::new(grid, rectangle.clone());

                (window, cursor_position)
            })
            .unzip();
        let borders = self.layout.borders();
        let cursor = cursors.into_iter().find_map(|cursor| cursor);
        let screen = Screen::new(windows, borders, cursor);

        // Set the global title
        let global_title_window = {
            let mode = self
                .context
                .mode()
                .map(|mode| mode.display())
                .unwrap_or_else(|| self.current_component().borrow().editor().display_mode());

            let mode = format!("[{}]", mode);

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

    pub fn handle_dispatches(&mut self, dispatches: Dispatches) -> Result<(), anyhow::Error> {
        for dispatch in dispatches.into_vec() {
            self.handle_dispatch(dispatch)?;
        }
        Ok(())
    }

    pub fn handle_dispatch(&mut self, dispatch: Dispatch) -> Result<(), anyhow::Error> {
        log::info!("App::handle_dispatch {}", dispatch.variant_name());
        match dispatch {
            Dispatch::CloseCurrentWindow => {
                self.close_current_window();
            }
            Dispatch::CloseCurrentWindowAndFocusParent => {
                self.close_current_window_and_focus_parent();
            }
            Dispatch::OpenSearchPrompt { scope, owner_id } => {
                self.open_search_prompt(scope, owner_id)?
            }
            Dispatch::OpenFile(path) => self.go_to_location(&Location {
                path,
                range: Range::default(),
            })?,

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
            Dispatch::ShowGlobalInfo(info) => self.show_global_info(info),
            Dispatch::SetQuickfixList(r#type) => {
                self.set_quickfix_list_type(Default::default(), r#type)?;
            }
            Dispatch::GotoQuickfixListItem(movement) => self.goto_quickfix_list_item(movement)?,
            Dispatch::GotoSelectionHistoryContiguous(movement) => {
                self.goto_selection_history_contiguous(movement)?;
            }
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
            Dispatch::GotoLocation(location) => self.go_to_location(&location)?,
            Dispatch::GlobalSearch => self.global_search()?,
            Dispatch::OpenMoveToIndexPrompt => self.open_move_to_index_prompt()?,
            Dispatch::RunCommand(command) => self.run_command(command)?,
            Dispatch::QuitAll => self.quit_all()?,
            Dispatch::OpenCommandPrompt => self.open_command_prompt()?,
            Dispatch::SaveQuitAll => self.save_quit_all()?,
            Dispatch::RevealInExplorer(path) => self.reveal_path_in_explorer(&path)?,
            Dispatch::OpenYesNoPrompt(prompt) => self.open_yes_no_prompt(prompt)?,
            Dispatch::OpenMoveFilePrompt(path) => self.open_move_file_prompt(path)?,
            Dispatch::OpenAddPathPrompt(path) => self.open_add_path_prompt(path)?,
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
            Dispatch::OpenInsideOtherPromptOpen => self.open_inside_other_prompt_open()?,
            Dispatch::OpenInsideOtherPromptClose { open } => {
                self.open_inside_other_prompt_close(open)?
            }
            Dispatch::OpenOmitPrompt {
                kind,
                target,
                make_mechanism,
            } => self.open_omit_prompt(kind, target, make_mechanism)?,

            Dispatch::LspExecuteCommand { command, params } => self
                .lsp_manager
                .workspace_execute_command(params, command)?,
            Dispatch::UpdateSelectionSet {
                selection_set,
                kind,
                store_history,
            } => self.update_selection_set(
                kind,
                UpdateSelectionSetSource::SelectionSet(selection_set),
                store_history,
            )?,
            Dispatch::UpdateLocalSearchConfig {
                update,
                owner_id,
                scope,
                show_config_after_enter,
            } => {
                self.update_local_search_config(update, owner_id, scope, show_config_after_enter)?
            }
            Dispatch::UpdateGlobalSearchConfig { owner_id, update } => {
                self.update_global_search_config(owner_id, update)?;
            }
            Dispatch::OpenSetGlobalSearchFilterGlobPrompt {
                owner_id,
                filter_glob,
            } => self.open_set_global_search_filter_glob_prompt(owner_id, filter_glob)?,
            Dispatch::ShowSearchConfig { owner_id, scope } => {
                self.show_search_config(owner_id, scope)
            }
            Dispatch::OpenUpdateReplacementPrompt { owner_id, scope } => {
                self.open_update_replacement_prompt(owner_id, scope)?
            }
            Dispatch::OpenUpdateSearchPrompt { owner_id, scope } => {
                self.open_update_search_prompt(owner_id, scope)?
            }
            Dispatch::Replace { scope } => match scope {
                Scope::Local => self.handle_dispatch_editor(ReplacePattern {
                    config: self.context.local_search_config().clone(),
                })?,
                Scope::Global => self.global_replace()?,
            },
            Dispatch::GoToPreviousSelection => self.go_to_previous_selection()?,
            Dispatch::GoToNextSelection => self.go_to_next_selection()?,
            Dispatch::HandleLspNotification(notification) => {
                self.handle_lsp_notification(notification)?
            }
            Dispatch::SetTheme(theme) => {
                let context = std::mem::take(&mut self.context);
                self.context = context.set_theme(theme);
            }
            Dispatch::HandleKeyEvents(key_events) => self.handle_key_events(key_events)?,
            Dispatch::ToSuggestiveEditor(dispatch) => {
                self.handle_dispatch_suggestive_editor(dispatch)?
            }
            Dispatch::CloseDropdown => self.layout.close_dropdown(),
            Dispatch::CloseEditorInfo => self.layout.close_editor_info(),
            Dispatch::RenderDropdown { render } => {
                if let Some(dropdown) = self.layout.open_dropdown() {
                    self.render_dropdown(dropdown, render)?
                }
            }
            Dispatch::OpenPrompt(prompt_config) => self.open_prompt(prompt_config)?,
            Dispatch::ShowEditorInfo(info) => self.show_editor_info(info)?,
            Dispatch::ReceiveCodeActions(code_actions) => {
                self.open_code_actions_prompt(code_actions)?;
            }
            Dispatch::OscillateWindow => self.layout.cycle_window(),
        }
        Ok(())
    }

    pub fn current_component(&self) -> Rc<RefCell<dyn Component>> {
        self.layout.get_current_component()
    }

    fn close_current_window(&mut self) {
        self.layout.close_current_window()
    }

    fn local_search(&mut self, owner_id: Option<ComponentId>) -> anyhow::Result<()> {
        let config = self.context.local_search_config();
        let search = config.search();
        if !search.is_empty() {
            self.handle_dispatch_editor_custom(
                SetSelectionMode(SelectionMode::Find {
                    search: Search {
                        mode: config.mode,
                        search,
                    },
                }),
                owner_id
                    .and_then(|owner_id| self.get_component_by_id(owner_id))
                    .unwrap_or_else(|| self.current_component()),
            )?;
        }

        Ok(())
    }

    fn resize(&mut self, dimension: Dimension) {
        self.layout
            .set_terminal_dimension(dimension.decrement_height(GLOBAL_TITLE_BAR_HEIGHT));
    }

    fn open_move_to_index_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: "Move to index".to_string(),
            history: vec![],
            on_enter: DispatchPrompt::MoveSelectionByIndex,
            items: vec![],
            enter_selects_first_matching_item: false,
        })
    }

    fn open_rename_prompt(
        &mut self,
        params: RequestParams,
        current_name: Option<String>,
    ) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: "Rename".to_string(),
            history: current_name.into_iter().collect_vec(),
            on_enter: DispatchPrompt::RenameSymbol { params },
            items: vec![],
            enter_selects_first_matching_item: false,
        })
    }

    fn open_search_prompt(&mut self, scope: Scope, owner_id: ComponentId) -> anyhow::Result<()> {
        let config = self.context.get_local_search_config(scope);
        let mode = config.mode;
        self.open_prompt(PromptConfig {
            title: format!("{:?} search ({})", scope, mode.display()),
            history: config.searches(),
            items: self.words(),
            on_enter: DispatchPrompt::UpdateLocalSearchConfigSearch {
                scope,
                owner_id,
                show_config_after_enter: false,
            },
            enter_selects_first_matching_item: false,
        })
    }

    fn open_inside_other_prompt_open(&mut self) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: "Inside (other): Open".to_string(),
            history: Vec::new(),
            on_enter: DispatchPrompt::OpenInsideOtherPromptClose,
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        })
    }

    fn open_inside_other_prompt_close(&mut self, open: String) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: format!("Inside (other, open = '{}'): Close", open),
            history: Vec::new(),
            on_enter: DispatchPrompt::EnterInsideMode { open },
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        })
    }

    fn open_add_path_prompt(&mut self, path: CanonicalizedPath) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: "Add path".to_string(),
            history: [path.display_absolute()].to_vec(),
            on_enter: DispatchPrompt::AddPath,
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        })
    }

    fn open_move_file_prompt(&mut self, path: CanonicalizedPath) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: "Move file".to_string(),
            history: [path.display_absolute()].to_vec(),
            on_enter: DispatchPrompt::MovePath { from: path },
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        })
    }

    fn open_symbol_picker(
        &mut self,
        _component_id: ComponentId,
        symbols: Symbols,
    ) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: "Symbols".to_string(),
            history: vec![],
            items: symbols
                .symbols
                .clone()
                .into_iter()
                .map(|symbol| symbol.into())
                .collect_vec(),
            on_enter: DispatchPrompt::SelectSymbol { symbols },
            enter_selects_first_matching_item: true,
        })
    }

    fn open_command_prompt(&mut self) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: "Command".to_string(),
            history: vec![],
            on_enter: DispatchPrompt::RunCommand,
            items: crate::command::COMMANDS
                .iter()
                .flat_map(|command| command.to_dropdown_items())
                .collect(),
            enter_selects_first_matching_item: true,
        })
    }

    fn open_file_picker(&mut self, kind: FilePickerKind) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        self.open_prompt(PromptConfig {
            title: format!("Open file: {}", kind.display()),
            history: vec![],
            on_enter: DispatchPrompt::OpenFile { working_directory },
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
                .map(|path| path.into())
                .collect_vec()
            },
            enter_selects_first_matching_item: true,
        })
    }

    fn open_file_custom(
        &mut self,
        path: &CanonicalizedPath,
        focus_editor: bool,
    ) -> anyhow::Result<Rc<RefCell<dyn Component>>> {
        // Check if the file is opened before
        // so that we won't notify the LSP twice
        if let Some(matching_editor) = self.layout.open_file(path, focus_editor) {
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
            self.layout
                .replace_and_focus_current_suggestive_editor(component.clone());
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

    fn open_file(
        &mut self,
        path: &CanonicalizedPath,
        focus_editor: bool,
    ) -> anyhow::Result<Rc<RefCell<dyn Component>>> {
        self.open_file_custom(path, focus_editor)
    }

    fn get_suggestive_editor(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<Rc<RefCell<SuggestiveEditor>>> {
        self.layout.get_suggestive_editor(component_id)
    }

    pub fn handle_lsp_notification(&mut self, notification: LspNotification) -> anyhow::Result<()> {
        match notification {
            LspNotification::Hover(hover) => self.show_editor_info(Info::new(
                "Hover Info".to_string(),
                hover.contents.join("\n\n"),
            )),
            LspNotification::Definition(context, response) => {
                match response {
                    GotoDefinitionResponse::Single(location) => self.go_to_location(&location)?,
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
            LspNotification::PrepareRenameResponse(context, response) => {
                let editor = self.get_suggestive_editor(context.component_id)?;
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
                            buffer.slice(&range.into())
                        })
                        .transpose()
                        .unwrap_or_default()
                        .map(|rope| rope.to_string());
                    (params, current_name)
                };
                if let Some(params) = params {
                    self.open_rename_prompt(params, current_name)?;
                }

                Ok(())
            }
            LspNotification::Error(error) => {
                self.show_global_info(Info::new("LSP Error".to_string(), error));
                Ok(())
            }
            LspNotification::WorkspaceEdit(workspace_edit) => {
                self.apply_workspace_edit(workspace_edit)
            }
            LspNotification::CodeAction(context, code_actions) => {
                if context.component_id == self.layout.get_current_component().borrow().id() {
                    self.handle_dispatch(Dispatch::ReceiveCodeActions(code_actions))?;
                }
                Ok(())
            }
            LspNotification::SignatureHelp(signature_help) => {
                if let Some(info) = signature_help.and_then(|s| s.into_info()) {
                    self.show_editor_info(info)?;
                } else {
                    self.hide_editor_info()
                }
                Ok(())
            }
            LspNotification::Symbols(context, symbols) => {
                self.open_symbol_picker(context.component_id, symbols)?;
                Ok(())
            }
        }
    }

    fn update_diagnostics(
        &mut self,
        path: CanonicalizedPath,
        diagnostics: Vec<lsp_types::Diagnostic>,
    ) -> anyhow::Result<()> {
        let component = self.open_file_custom(&path, false)?;

        component
            .borrow_mut()
            .editor_mut()
            .buffer_mut()
            .set_diagnostics(diagnostics);
        Ok(())
    }

    pub fn get_quickfix_list(&self) -> Option<QuickfixList> {
        self.context.quickfix_list_state().as_ref().map(|state| {
            QuickfixList::new(
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
        self.layout.show_global_info(info).unwrap_or_else(|err| {
            log::error!("Error showing info: {:?}", err);
        });
    }

    fn go_to_location(&mut self, Location { path, range }: &Location) -> Result<(), anyhow::Error> {
        self.update_selection_set(
            SelectionSetHistoryKind::Path(path.clone()),
            UpdateSelectionSetSource::PositionRange(range.clone()),
            true,
        )
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
                self.context
                    .set_quickfix_list_source(QuickfixListSource::Diagnostic(severity_range));
            }
            QuickfixListType::Items(items) => {
                self.layout.clear_quickfix_list_items();
                items
                    .into_iter()
                    .group_by(|item| item.location().path.clone())
                    .into_iter()
                    .map(|(path, items)| -> anyhow::Result<()> {
                        let editor = self.open_file_custom(&path, false)?;
                        editor
                            .borrow_mut()
                            .editor_mut()
                            .buffer_mut()
                            .update_quickfix_list_items(items.collect_vec());
                        Ok(())
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                self.context
                    .set_quickfix_list_source(QuickfixListSource::Custom);
            }
            QuickfixListType::Bookmark => {
                self.context
                    .set_quickfix_list_source(QuickfixListSource::Bookmark);
            }
        }
        match context.scope {
            None | Some(Scope::Global) => {
                self.goto_quickfix_list_item(Movement::Current)?;
                Ok(())
            }
            Some(Scope::Local) => self.handle_dispatch(Dispatch::ToEditor(SetSelectionMode(
                SelectionMode::LocalQuickfix { title },
            ))),
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
        let locations = match config.mode {
            LocalSearchConfigMode::Regex(regex) => {
                list::grep::run(&config.search(), walk_builder_config, regex)
            }
            LocalSearchConfigMode::AstGrep => {
                list::ast_grep::run(config.search().clone(), walk_builder_config)
            }
            LocalSearchConfigMode::CaseAgnostic => {
                list::case_agnostic::run(config.search().clone(), walk_builder_config)
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
        self.add_path_parent(&to)?;
        fs::rename(from.clone(), to.clone())?;
        self.layout.refresh_file_explorer(&self.working_directory)?;
        let to = to.try_into()?;
        self.reveal_path_in_explorer(&to)?;
        self.lsp_manager.document_did_rename(from, to)?;
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
        self.layout.refresh_file_explorer(&self.working_directory)?;
        self.reveal_path_in_explorer(&path.try_into()?)?;

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
            })?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn get_current_selected_texts(&self) -> Vec<String> {
        let _content = self.current_component().borrow().content();
        self.current_component()
            .borrow()
            .editor()
            .get_selected_texts()
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
        let dispatches = component
            .borrow_mut()
            .editor_mut()
            .apply_dispatch(&mut self.context, dispatch_editor)?;

        self.handle_dispatches(dispatches)?;
        Ok(())
    }

    fn get_repo_git_hunks(&mut self) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        let repo = git::GitRepo::try_from(&working_directory)?;
        let diffs = repo.diffs()?;
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

    fn set_global_title(&mut self, title: String) {
        self.global_title = Some(title)
    }

    pub fn set_syntax_highlight_request_sender(&mut self, sender: Sender<SyntaxHighlightRequest>) {
        self.syntax_highlight_request_sender = Some(sender);
    }

    #[cfg(test)]
    pub(crate) fn get_current_file_path(&self) -> Option<CanonicalizedPath> {
        self.current_component().borrow().path()
    }

    fn set_global_mode(&mut self, mode: Option<GlobalMode>) {
        self.context.set_mode(mode);
    }

    fn open_omit_prompt(
        &mut self,
        kind: FilterKind,
        target: FilterTarget,
        make_mechanism: MakeFilterMechanism,
    ) -> anyhow::Result<()> {
        self.open_prompt(PromptConfig {
            title: format!(
                "Omit: {:?} selection by {:?} matching {:?}",
                kind, target, make_mechanism
            ),
            history: Vec::new(),
            on_enter: DispatchPrompt::PushFilter {
                kind,
                target,
                make_mechanism,
            },
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        })
    }

    fn update_selection_set(
        &mut self,
        kind: SelectionSetHistoryKind,
        source: UpdateSelectionSetSource,
        store_history: bool,
    ) -> anyhow::Result<()> {
        let component = self.layout.get_current_component();
        let new_to_old = SelectionSetHistory {
            kind: component
                .borrow()
                .path()
                .map(SelectionSetHistoryKind::Path)
                .unwrap_or_else(|| SelectionSetHistoryKind::ComponentId(None)),
            selection_set: component.borrow().editor().selection_set.clone(),
        };
        if let Some(new_component) = match &kind {
            SelectionSetHistoryKind::Path(path) => Some(self.open_file(path, true)?),
            SelectionSetHistoryKind::ComponentId(Some(id)) => self.layout.get_component_by_id(id),
            _ => None,
        } {
            let selection_set = match source {
                UpdateSelectionSetSource::PositionRange(position_range) => new_component
                    .borrow_mut()
                    .editor_mut()
                    .position_range_to_selection_set(position_range)?,
                UpdateSelectionSetSource::SelectionSet(selection_set) => selection_set,
            };
            new_component
                .borrow_mut()
                .editor_mut()
                .__update_selection_set_for_real(selection_set.clone());
            let old_new = crate::undo_tree::OldNew {
                old_to_new: SelectionSetHistory {
                    kind: kind.clone(),
                    selection_set,
                },
                new_to_old,
            };
            if store_history {
                self.selection_set_history.push(old_new.clone());
            }
        }
        Ok(())
    }

    fn goto_selection_history_contiguous(&mut self, movement: Movement) -> anyhow::Result<()> {
        match movement {
            Movement::Next => {
                if let Some(go_to_selection_set_history) = self.selection_set_history.redo() {
                    self.go_to_selection_set_history(go_to_selection_set_history)?;
                }
                // self.undo_tree.redo(&mut self.layout)?;
            }
            Movement::Previous => {
                if let Some(file_selection_set) = self.selection_set_history.undo() {
                    self.go_to_selection_set_history(file_selection_set)?;
                }
                // self.undo_tree.undo(&mut self.layout)?;
            }

            _ => {}
        }
        Ok(())
    }

    #[cfg(test)]
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
        Some(
            self.components()
                .into_iter()
                .find(|component| component.component().borrow().id() == id)?
                .component(),
        )
    }

    fn open_set_global_search_filter_glob_prompt(
        &mut self,
        owner_id: ComponentId,
        filter_glob: GlobalSearchFilterGlob,
    ) -> anyhow::Result<()> {
        let _current_component = self.current_component().clone();
        let config = self.context.global_search_config();
        let history = match filter_glob {
            GlobalSearchFilterGlob::Include => config.include_globs(),
            GlobalSearchFilterGlob::Exclude => config.exclude_globs(),
        };
        self.open_prompt(PromptConfig {
            title: format!("Set global search {:?} files glob", filter_glob),
            history,
            on_enter: DispatchPrompt::GlobalSearchConfigSetGlob {
                owner_id,
                filter_glob,
            },
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        })
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
                        show_config_after_enter: true,
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
            LocalSearchConfigMode::CaseAgnostic => None,
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
                                "a",
                                "AST Grep".to_string(),
                                LocalSearchConfigMode::AstGrep,
                                local_search_config.mode == LocalSearchConfigMode::AstGrep,
                            ),
                            update_mode_keymap(
                                "c",
                                "Case Agnostic".to_string(),
                                LocalSearchConfigMode::CaseAgnostic,
                                local_search_config.mode == LocalSearchConfigMode::CaseAgnostic,
                            ),
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
                                    "i",
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

    fn open_update_replacement_prompt(
        &mut self,
        owner_id: ComponentId,
        scope: Scope,
    ) -> Result<(), anyhow::Error> {
        self.open_prompt(PromptConfig {
            title: format!("Set Replace ({:?})", scope),
            history: self.context.get_local_search_config(scope).replacements(),
            on_enter: DispatchPrompt::UpdateLocalSearchConfigReplacement { owner_id, scope },
            items: Vec::new(),
            enter_selects_first_matching_item: false,
        })
    }

    fn open_update_search_prompt(
        &mut self,
        owner_id: ComponentId,
        scope: Scope,
    ) -> Result<(), anyhow::Error> {
        self.open_prompt(PromptConfig {
            title: format!("Set Search ({:?})", scope),
            history: self.context.get_local_search_config(scope).searches(),
            on_enter: DispatchPrompt::UpdateLocalSearchConfigSearch {
                scope,
                owner_id,
                show_config_after_enter: true,
            },
            items: self.words(),
            enter_selects_first_matching_item: false,
        })
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

    fn go_to_selection_set_history(
        &mut self,
        selection_set_history: SelectionSetHistory,
    ) -> anyhow::Result<()> {
        match selection_set_history.kind {
            SelectionSetHistoryKind::Path(path) => self
                .layout
                .open_file_with_selection(&path, selection_set_history.selection_set)?,
            SelectionSetHistoryKind::ComponentId(Some(_)) => {
                return Err(anyhow::anyhow!(
                    "App::go_to_selection_set_history This is not handled yet"
                ))
            }
            _ => {}
        };
        Ok(())
    }

    fn go_to_previous_selection(&mut self) -> anyhow::Result<()> {
        if let Some(file_selection_set) = self.selection_set_history.undo() {
            self.go_to_selection_set_history(file_selection_set)?;
        }
        Ok(())
    }

    fn go_to_next_selection(&mut self) -> anyhow::Result<()> {
        if let Some(file_selection_set) = self.selection_set_history.redo() {
            self.go_to_selection_set_history(file_selection_set)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn get_current_component_content(&self) -> String {
        self.current_component().borrow().editor().content()
    }

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
        let component = self
            .layout
            .get_component_by_kind(ComponentKind::SuggestiveEditor)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "App::handle_dispatch_suggestive_editor Cannot find suggestive editor"
                )
            })?;
        let dispatches = component
            .borrow_mut()
            .as_any_mut()
            .downcast_mut::<SuggestiveEditor>()
            .ok_or_else(|| {
                anyhow::anyhow!("App::handle_dispatch_suggestive_editor Failed to downcast")
            })?
            .handle_dispatch(dispatch)?;
        self.handle_dispatches(dispatches)
    }

    #[cfg(test)]
    pub(crate) fn completion_dropdown_is_open(&self) -> bool {
        self.layout.completion_dropdown_is_open()
    }

    #[cfg(test)]
    pub(crate) fn current_completion_dropdown(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.current_completion_dropdown()
    }

    fn open_prompt(&mut self, prompt_config: PromptConfig) -> anyhow::Result<()> {
        let (prompt, dispatches) = Prompt::new(prompt_config);

        self.layout
            .add_and_focus_prompt(ComponentKind::Prompt, Rc::new(RefCell::new(prompt)));
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
                self.layout.show_dropdown_info(info)?;
            }
            _ => self.layout.hide_dropdown_info(),
        }
        self.handle_dispatches(dispatches)
    }

    #[cfg(test)]
    pub(crate) fn get_dropdown_infos_count(&self) -> usize {
        self.layout.get_dropdown_infos_count()
    }

    pub fn render_quickfix_list(&mut self, quickfix_list: QuickfixList) -> anyhow::Result<()> {
        let dispatches = self.layout.show_quickfix_list(quickfix_list)?;
        self.handle_dispatches(dispatches)
    }

    fn show_editor_info(&mut self, info: Info) -> anyhow::Result<()> {
        self.layout.show_editor_info(info)
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
        let dispatches = self.layout.reveal_path_in_explorer(path)?;
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
        let component = self.layout.get_current_component();
        let params = component.borrow().editor().get_request_params();
        self.open_prompt(PromptConfig {
            history: Vec::new(),
            on_enter: DispatchPrompt::Null,
            items: code_actions
                .into_iter()
                .map(move |code_action| code_action.into_dropdown_item(params.clone()))
                .collect(),
            title: "Code Actions".to_string(),
            enter_selects_first_matching_item: true,
        })?;
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

#[must_use]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Dispatches(Vec<Dispatch>);
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
}

#[must_use]
#[derive(Clone, Debug, PartialEq, Eq, name_variant::NamedVariant)]
/// Dispatch are for child component to request action from the root node
pub enum Dispatch {
    SetTheme(Theme),
    CloseCurrentWindow,
    OpenFilePicker(FilePickerKind),
    OpenSearchPrompt {
        scope: Scope,
        owner_id: ComponentId,
    },
    OpenFile(CanonicalizedPath),
    ShowGlobalInfo(Info),
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
    ApplyWorkspaceEdit(WorkspaceEdit),
    ShowKeymapLegend(KeymapLegendConfig),
    RemainOnlyCurrentComponent,

    #[cfg(test)]
    /// Used for testing
    Custom(String),
    ToEditor(DispatchEditor),
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
    HandleKeyEvents(Vec<event::KeyEvent>),
    GetRepoGitHunks,
    SaveAll,
    TerminalDimensionChanged(Dimension),
    SetGlobalTitle(String),
    OpenInsideOtherPromptOpen,
    OpenInsideOtherPromptClose {
        open: String,
    },
    OpenOmitPrompt {
        kind: FilterKind,
        target: FilterTarget,
        make_mechanism: MakeFilterMechanism,
    },
    LspExecuteCommand {
        params: RequestParams,
        command: crate::lsp::code_action::Command,
    },
    UpdateSelectionSet {
        selection_set: SelectionSet,
        kind: SelectionSetHistoryKind,
        store_history: bool,
    },
    GotoSelectionHistoryContiguous(Movement),
    UpdateLocalSearchConfig {
        owner_id: ComponentId,
        update: LocalSearchConfigUpdate,
        scope: Scope,
        show_config_after_enter: bool,
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
    GoToPreviousSelection,
    GoToNextSelection,
    HandleLspNotification(LspNotification),
    ToSuggestiveEditor(DispatchSuggestiveEditor),
    CloseDropdown,
    RenderDropdown {
        render: DropdownRender,
    },
    OpenPrompt(PromptConfig),
    ShowEditorInfo(Info),
    ReceiveCodeActions(Vec<crate::lsp::code_action::CodeAction>),
    OscillateWindow,
    CloseCurrentWindowAndFocusParent,
    CloseEditorInfo,
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
    Mode(LocalSearchConfigMode),
    Replacement(String),
    Search(String),
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

enum UpdateSelectionSetSource {
    PositionRange(Range<Position>),
    SelectionSet(SelectionSet),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MakeFilterMechanism {
    Literal,
    Regex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchPrompt {
    PushFilter {
        kind: FilterKind,
        target: FilterTarget,
        make_mechanism: MakeFilterMechanism,
    },
    GlobalSearchConfigSetGlob {
        owner_id: ComponentId,
        filter_glob: GlobalSearchFilterGlob,
    },
    MoveSelectionByIndex,
    RenameSymbol {
        params: RequestParams,
    },
    UpdateLocalSearchConfigSearch {
        scope: Scope,
        owner_id: ComponentId,
        show_config_after_enter: bool,
    },
    OpenInsideOtherPromptClose,
    EnterInsideMode {
        open: String,
    },
    AddPath,
    MovePath {
        from: CanonicalizedPath,
    },
    Null,
    // TODO: remove the following variants
    // Because the following action already embeds dispatches
    SelectSymbol {
        symbols: Symbols,
    },
    RunCommand,
    OpenFile {
        working_directory: CanonicalizedPath,
    },
    UpdateLocalSearchConfigReplacement {
        owner_id: ComponentId,
        scope: Scope,
    },
    SetContent,
}
impl DispatchPrompt {
    pub fn to_dispatches(&self, text: &str) -> anyhow::Result<Dispatches> {
        match self.clone() {
            DispatchPrompt::PushFilter {
                kind,
                target,
                make_mechanism: mechanism,
            } => {
                let mechanism = match mechanism {
                    MakeFilterMechanism::Literal => FilterMechanism::Literal(text.to_string()),
                    MakeFilterMechanism::Regex => FilterMechanism::Regex(regex::Regex::new(text)?),
                };
                Ok(Dispatches::new(
                    [Dispatch::ToEditor(FilterPush(Filter::new(
                        kind, target, mechanism,
                    )))]
                    .to_vec(),
                ))
            }
            DispatchPrompt::GlobalSearchConfigSetGlob {
                owner_id,
                filter_glob,
            } => Ok(Dispatches::new(
                [Dispatch::UpdateGlobalSearchConfig {
                    owner_id,
                    update: GlobalSearchConfigUpdate::SetGlob(filter_glob, text.to_string()),
                }]
                .to_vec(),
            )),
            DispatchPrompt::MoveSelectionByIndex => {
                let index = text.parse::<usize>()?.saturating_sub(1);
                Ok(Dispatches::new(
                    [Dispatch::ToEditor(MoveSelection(Movement::Index(index)))].to_vec(),
                ))
            }
            DispatchPrompt::RenameSymbol { params } => {
                Ok(Dispatches::new(vec![Dispatch::RenameSymbol {
                    params: params.clone(),
                    new_name: text.to_string(),
                }]))
            }
            DispatchPrompt::UpdateLocalSearchConfigSearch {
                scope,
                owner_id,
                show_config_after_enter,
            } => Ok(Dispatches::new(
                [Dispatch::UpdateLocalSearchConfig {
                    owner_id,
                    update: LocalSearchConfigUpdate::Search(text.to_string()),
                    scope,
                    show_config_after_enter,
                }]
                .to_vec(),
            )),
            DispatchPrompt::OpenInsideOtherPromptClose => Ok(Dispatches::new(
                [Dispatch::OpenInsideOtherPromptClose {
                    open: text.to_owned(),
                }]
                .to_vec(),
            )),
            DispatchPrompt::EnterInsideMode { open } => Ok(Dispatches::new(
                [Dispatch::ToEditor(EnterInsideMode(InsideKind::Other {
                    open: open.clone(),
                    close: text.to_owned(),
                }))]
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
            DispatchPrompt::RunCommand => Ok(Dispatches::new(
                [Dispatch::RunCommand(text.to_string())]
                    .into_iter()
                    .collect(),
            )),
            DispatchPrompt::OpenFile { working_directory } => {
                let path = working_directory.join(text)?;
                Ok(Dispatches::new(vec![Dispatch::OpenFile(path)]))
            }
            DispatchPrompt::UpdateLocalSearchConfigReplacement { owner_id, scope } => {
                Ok(Dispatches::new(
                    [Dispatch::UpdateLocalSearchConfig {
                        owner_id,
                        scope,
                        update: LocalSearchConfigUpdate::Replacement(text.to_owned()),
                        show_config_after_enter: true,
                    }]
                    .to_vec(),
                ))
            }
            DispatchPrompt::SetContent => Ok(Dispatches::new(
                [Dispatch::ToEditor(SetContent(text.to_string()))].to_vec(),
            )),
            DispatchPrompt::Null => Ok(Default::default()),
        }
    }
}
