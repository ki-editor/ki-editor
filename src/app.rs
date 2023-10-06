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
        keymap_legend::{Keymap, KeymapLegendConfig},
        prompt::{Prompt, PromptConfig},
        suggestive_editor::{Info, SuggestiveEditor, SuggestiveEditorFilter},
    },
    context::{Context, GlobalMode, Search, SearchKind},
    frontend::frontend::Frontend,
    grid::Grid,
    layout::Layout,
    list,
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
    selection::SelectionMode,
    syntax_highlight::SyntaxHighlightRequest,
    themes::VSCODE_LIGHT,
};

pub struct App<T: Frontend> {
    context: Context,

    buffers: Vec<Rc<RefCell<Buffer>>>,

    sender: Sender<AppMessage>,

    /// Used for receiving message from various sources:
    /// - Events from crossterm
    /// - Notifications from language server
    receiver: Receiver<AppMessage>,

    lsp_manager: LspManager,
    enable_lsp: bool,

    working_directory: CanonicalizedPath,

    layout: Layout,

    frontend: Arc<Mutex<T>>,

    syntax_highlight_request_sender: Option<Sender<SyntaxHighlightRequest>>,
}

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
            context: Context::new(),
            buffers: Vec::new(),
            receiver,
            lsp_manager: LspManager::new(sender.clone(), working_directory.clone()),
            enable_lsp: true,
            sender,
            layout: Layout::new(dimension, &working_directory)?,
            working_directory,
            frontend,
            syntax_highlight_request_sender: None,
        };
        Ok(app)
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
            }
            .unwrap_or_else(|e| {
                self.show_info("ERROR", vec![e.to_string()]);
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
                        .unwrap_or_else(|e| self.show_info("ERROR", vec![e.to_string()]));
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
        // Recalculate layout before each render
        self.layout.recalculate_layout();

        const GLOBAL_TITLE_BAR_HEIGHT: u16 = 1;

        // Generate layout
        let dimension = self.layout.terminal_dimension();
        let grid = Grid::new(Dimension {
            height: dimension.height.saturating_sub(GLOBAL_TITLE_BAR_HEIGHT),
            width: dimension.width,
        });

        let theme = self.context.theme().clone();

        // Render every window
        let (grid, cursor) = self
            .components()
            .into_iter()
            .map(|component| {
                let component = component.borrow();

                let rectangle = component
                    .rectangle()
                    .clamp_top(GLOBAL_TITLE_BAR_HEIGHT as usize);

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

                (grid, rectangle.clone(), cursor_position, component.title())
            })
            .fold(
                (grid, None),
                |(grid, current_cursor_point), (component_grid, rectangle, cursor_point, title)| {
                    {
                        let title_rectangle = rectangle.move_up(1).set_height(1);
                        let title_grid = Grid::new(title_rectangle.dimension()).set_line(
                            0,
                            &title,
                            &theme.ui.window_title,
                        );
                        (
                            grid.update(&component_grid, &rectangle)
                                // Set title
                                .update(&title_grid, &title_rectangle),
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

            let title = if let Some(current_branch) = self.current_branch() {
                format!(
                    "{} ({}) {}",
                    self.working_directory.display(),
                    current_branch,
                    mode
                )
            } else {
                format!("{} {}", self.working_directory.display(), mode)
            };

            Grid::new(Dimension {
                height: 1,
                width: dimension.width,
            })
            .set_line(0, &title, &self.context.theme().ui.global_title)
        };

        self.render_grid(grid.merge_vertical(global_title_grid), cursor)?;

        Ok(())
    }

    fn current_branch(&self) -> Option<String> {
        // Open the repository
        let repo = git2::Repository::open(self.working_directory.display()).ok()?;

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

    fn handle_dispatches(&mut self, dispatches: Vec<Dispatch>) -> Result<(), anyhow::Error> {
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
            Dispatch::SetSearch(search) => self.set_search(search),
            Dispatch::OpenSearchPrompt(search_kind) => self.open_search_prompt(search_kind),
            Dispatch::OpenFile { path } => {
                self.open_file(&path, true)?;
            }

            Dispatch::OpenFilePicker(kind) => {
                self.open_file_picker(kind)?;
            }
            Dispatch::RequestCompletion(params) => {
                self.lsp_manager.request_completion(params)?;
            }
            Dispatch::RequestReferences(params) => self.lsp_manager.request_references(params)?,
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
            Dispatch::RequestCodeAction(action) => {
                self.lsp_manager.request_code_action(action)?;
            }
            Dispatch::RequestSignatureHelp(params) => {
                self.lsp_manager.request_signature_help(params)?;
            }
            Dispatch::RequestDocumentSymbols(params) => {
                self.lsp_manager.request_document_symbols(params)?;
            }
            Dispatch::DocumentDidChange { path, content, .. } => {
                if let Some(path) = path {
                    self.lsp_manager.document_did_change(path, content)?;
                }
            }
            Dispatch::DocumentDidSave { path } => {
                self.lsp_manager.document_did_save(path)?;
            }
            Dispatch::ShowInfo { title, content } => self.show_info(&title, content),
            Dispatch::SetQuickfixList(r#type) => self.set_quickfix_list_type(r#type)?,
            Dispatch::GotoQuickfixListItem(direction) => self.goto_quickfix_list_item(direction)?,
            Dispatch::GotoOpenedEditor(direction) => self.layout.goto_opened_editor(direction),
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
            Dispatch::OpenGlobalSearchPrompt(search_kind) => {
                self.open_global_search_prompt(search_kind)
            }
            Dispatch::GlobalSearch(search) => self.global_search(search)?,
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
            Dispatch::SetGlobalMode(mode) => self.context.set_mode(mode),
            Dispatch::HandleKeyEvent(key_event) => {
                self.handle_event(Event::Key(key_event))?;
            }
        }
        Ok(())
    }

    fn current_component(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.current_component()
    }

    fn close_current_window(&mut self, change_focused_to: Option<ComponentId>) {
        self.layout.close_current_window(change_focused_to)
    }

    fn set_search(&mut self, search: Search) {
        self.context.set_search(search);
    }

    fn resize(&mut self, dimension: Dimension) {
        self.layout.set_terminal_dimension(dimension);
    }

    fn open_move_to_index_prompt(&mut self) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Move to index".to_string(),
            history: vec![],
            initial_text: None,
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
        });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_rename_prompt(&mut self, params: RequestParams, current_name: Option<String>) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Rename".to_string(),
            initial_text: current_name,
            history: vec![],
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                Ok(vec![Dispatch::RenameSymbol {
                    params: params.clone(),
                    new_name: text.to_string(),
                }])
            }),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: vec![],
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_global_search_prompt(&mut self, search_kind: SearchKind) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: format!("Global search ({})", search_kind.display()),
            initial_text: None,
            history: self
                .context
                .previous_searches()
                .iter()
                .map(|search| search.search.clone())
                .collect_vec(),
            owner: current_component.clone(),
            items: vec![],
            on_text_change: Box::new(|_, _| Ok(vec![])),
            on_enter: Box::new(move |text, _| {
                Ok([Dispatch::GlobalSearch(Search {
                    kind: search_kind,
                    search: text.to_string(),
                })]
                .to_vec())
            }),
        });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_search_prompt(&mut self, kind: SearchKind) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: format!("Search ({})", kind.display()),
            initial_text: None,
            history: self
                .context
                .previous_searches()
                .into_iter()
                .map(|search| search.search)
                .collect_vec(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| {
                let search = Search {
                    kind,
                    search: text.to_string(),
                };

                Ok([
                    Dispatch::SetSearch(search.clone()),
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        crate::selection::SelectionMode::Find { search },
                    )),
                ]
                .to_vec())
            }),
            on_text_change: Box::new(|_current_text, _owner| {
                // owner
                //     .borrow_mut()
                //     .editor_mut()
                //     .select_match(Direction::Forward, &Some(current_text.to_string()));
                Ok(vec![])
            }),
            items: current_component
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
                .unwrap_or_default(),
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_add_path_prompt(&mut self, path: CanonicalizedPath) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Add path".to_string(),
            history: Vec::new(),
            initial_text: Some(path.display()),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, _| Ok([Dispatch::AddPath(text.into())].to_vec())),
            on_text_change: Box::new(|_current_text, _owner| Ok(vec![])),
            items: Vec::new(),
        });

        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_move_file_prompt(&mut self, path: CanonicalizedPath) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Move file".to_string(),
            history: Vec::new(),
            initial_text: Some(path.display()),
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
                initial_text: None,
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
            initial_text: None,
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
            initial_text: None,
            owner: current_component,
            on_enter: Box::new(move |current_item, _| {
                let path = working_directory.join(current_item)?;
                Ok(vec![Dispatch::OpenFile { path }])
            }),
            on_text_change: Box::new(|_, _| Ok(vec![])),
            items: {
                match kind {
                    FilePickerKind::NonGitIgnored => {
                        list::non_gitignore_files::non_git_ignored_files(&self.working_directory)?
                    }
                    FilePickerKind::GitStatus => {
                        list::git_status_files::git_status_files(&self.working_directory)?
                    }
                    FilePickerKind::Opened => self
                        .layout
                        .get_opened_files()
                        .into_iter()
                        .filter_map(|path| path.display_relative_to(&self.working_directory).ok())
                        .collect_vec(),
                }
                .into_iter()
                .map(CompletionItem::from_label)
                .collect_vec()
            },
        });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
        Ok(())
    }

    pub fn open_file(
        &mut self,
        entry_path: &CanonicalizedPath,
        focus_editor: bool,
    ) -> anyhow::Result<Rc<RefCell<dyn Component>>> {
        // Check if the file is opened before
        // so that we won't notify the LSP twice
        if let Some(matching_editor) = self.layout.open_file(entry_path) {
            return Ok(matching_editor);
        } else {
        }

        let buffer = Buffer::from_path(entry_path)?;
        let language = buffer.language();
        let content = buffer.content();
        let buffer = Rc::new(RefCell::new(buffer));
        self.buffers.push(buffer.clone());
        let editor = SuggestiveEditor::from_buffer(buffer, SuggestiveEditorFilter::CurrentWord);
        let component_id = editor.id();
        let component = Rc::new(RefCell::new(editor));

        if focus_editor {
            self.layout
                .add_and_focus_suggestive_editor(component.clone());
        } else {
            self.layout.add_suggestive_editor(component.clone());
        }

        if let Some(language) = language {
            self.request_syntax_highlight(component_id, language, content)?;
        }

        if self.enable_lsp {
            self.lsp_manager.open_file(entry_path.clone())?;
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
                    .show_infos(
                        "Hover info",
                        [Info {
                            content: hover.contents.join("\n\n"),
                            decorations: Vec::new(),
                        }]
                        .to_vec(),
                    );
                Ok(())
            }
            LspNotification::Definition(context, response) => {
                match response {
                    GotoDefinitionResponse::Single(location) => self.go_to_location(&location)?,
                    GotoDefinitionResponse::Multiple(locations) => {
                        if locations.is_empty() {
                            self.show_info(
                                "Goto definition info",
                                vec!["No definitions found".to_string()],
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
                    self.buffers
                        .iter()
                        .filter_map(|buffer| buffer.borrow().path())
                        .collect::<Vec<_>>(),
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
                self.show_info("LSP error", vec![error]);
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

    fn show_info(&mut self, title: &str, contents: Vec<String>) {
        self.layout
            .show_info(title, contents)
            .unwrap_or_else(|err| {
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
                                vec![diagnostic.message()],
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
        match context.request_kind {
            None | Some(RequestKind::Global) => {
                self.layout
                    .show_quickfix_lists(self.context.quickfix_lists().clone());
                self.goto_quickfix_list_item(Movement::Next)
            }
            Some(RequestKind::Local) => self.handle_dispatch(Dispatch::DispatchEditor(
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

    fn global_search(&mut self, search: Search) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        let locations = match search.kind {
            SearchKind::Regex => list::grep::run(
                &search.search,
                working_directory.clone().into(),
                false,
                false,
            ),
            SearchKind::Literal => list::grep::run(
                &search.search,
                working_directory.clone().into(),
                true,
                false,
            ),
            SearchKind::AstGrep => {
                list::ast_grep::run(&search.search, working_directory.clone().into())
            }
            SearchKind::LiteralIgnoreCase => {
                list::grep::run(&search.search, working_directory.clone().into(), true, true)
            }
        }?;
        self.set_quickfix_list(
            ResponseContext::default().set_description("Global search"),
            QuickfixList::new(
                locations
                    .into_iter()
                    .map(|location| QuickfixListItem::new(location, vec![]))
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
            title: prompt.title,
            owner_id: prompt.owner_id,
            keymaps: [
                Keymap::new("y", "Yes", *prompt.yes),
                Keymap::new("n", "No", Dispatch::Null),
            ]
            .to_vec(),
        }))
    }

    fn delete_path(&mut self, path: &CanonicalizedPath) -> anyhow::Result<()> {
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        self.buffers.retain(|buffer| {
            buffer
                .borrow()
                .path()
                .as_ref()
                .map_or(true, |buffer_path| buffer_path != path)
        });
        self.layout.remove_suggestive_editor(path);
        self.layout.refresh_file_explorer(&self.working_directory)?;
        Ok(())
    }

    fn move_file(&mut self, from: CanonicalizedPath, to: PathBuf) -> anyhow::Result<()> {
        use std::fs;
        log::info!("move file from {} to {}", from.display(), to.display());
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
                theme: VSCODE_LIGHT,
            })?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn get_selected_texts(&mut self, path: &CanonicalizedPath) -> Vec<String> {
        self.layout
            .open_file(path)
            .map(|matching_editor| matching_editor.borrow().editor().get_selected_texts())
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub fn get_file_content(&mut self, path: &CanonicalizedPath) -> String {
        self.layout
            .open_file(path)
            .map(|matching_editor| matching_editor.borrow().content())
            .unwrap_or_default()
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
        if let Some(component) = self.current_component() {
            let dispatches = component
                .borrow_mut()
                .editor_mut()
                .apply_dispatch(&mut self.context, dispatch_editor)?;

            self.handle_dispatches(dispatches)?;
        }
        Ok(())
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Dispatch are for child component to request action from the root node
pub enum Dispatch {
    CloseCurrentWindow {
        change_focused_to: Option<ComponentId>,
    },
    SetSearch(Search),
    OpenFilePicker(FilePickerKind),
    OpenSearchPrompt(SearchKind),
    OpenFile {
        path: CanonicalizedPath,
    },
    ShowInfo {
        title: String,
        content: Vec<String>,
    },
    RequestCompletion(RequestParams),
    RequestSignatureHelp(RequestParams),
    RequestHover(RequestParams),
    RequestDefinitions(RequestParams),
    RequestDeclarations(RequestParams),
    RequestImplementations(RequestParams),
    RequestTypeDefinitions(RequestParams),
    RequestReferences(RequestParams),
    PrepareRename(RequestParams),
    RequestCodeAction(RequestParams),
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
    GotoOpenedEditor(Movement),
    ApplyWorkspaceEdit(WorkspaceEdit),
    ShowKeymapLegend(KeymapLegendConfig),
    CloseAllExceptMainPanel,

    #[cfg(test)]
    /// Used for testing
    Custom(&'static str),
    DispatchEditor(DispatchEditor),
    RequestDocumentSymbols(RequestParams),
    GotoLocation(Location),
    OpenGlobalSearchPrompt(SearchKind),
    GlobalSearch(Search),
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
    pub fn set_kind(self, request_kind: Option<RequestKind>) -> Self {
        Self {
            context: ResponseContext {
                request_kind,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestKind {
    Local,
    Global,
}

#[derive(Debug)]
pub enum AppMessage {
    LspNotification(LspNotification),
    Event(Event),
    QuitAll,
}
