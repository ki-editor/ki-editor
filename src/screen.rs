use anyhow::anyhow;

use event::event::Event;
use itertools::Itertools;
use my_proc_macros::{hex, key};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

use crate::{
    buffer::Buffer,
    canonicalized_path::CanonicalizedPath,
    components::{
        component::{Component, ComponentId, GetGridResult},
        editor::{Direction, DispatchEditor},
        keymap_legend::KeymapLegendConfig,
        prompt::{Prompt, PromptConfig},
        suggestive_editor::{SuggestiveEditor, SuggestiveEditorFilter},
    },
    context::{Context, GlobalMode, Search, SearchKind},
    frontend::frontend::Frontend,
    grid::{Grid, Style},
    layout::Layout,
    list,
    lsp::{
        completion::CompletionItem, diagnostic::Diagnostic,
        goto_definition_response::GotoDefinitionResponse, manager::LspManager,
        process::LspNotification, symbols::Symbols, workspace_edit::WorkspaceEdit,
    },
    position::Position,
    quickfix_list::{Location, QuickfixList, QuickfixListItem, QuickfixListType, QuickfixLists},
};

pub struct Screen<T: Frontend> {
    context: Context,

    buffers: Vec<Rc<RefCell<Buffer>>>,

    sender: Sender<ScreenMessage>,

    /// Used for receiving message from various sources:
    /// - Events from crossterm
    /// - Notifications from language server
    receiver: Receiver<ScreenMessage>,

    lsp_manager: LspManager,

    diagnostics: HashMap<CanonicalizedPath, Vec<Diagnostic>>,

    quickfix_lists: Rc<RefCell<QuickfixLists>>,

    working_directory: CanonicalizedPath,

    layout: Layout,

    frontend: Arc<Mutex<T>>,
}

impl<T: Frontend> Screen<T> {
    pub fn new(
        frontend: Arc<Mutex<T>>,
        working_directory: CanonicalizedPath,
    ) -> anyhow::Result<Screen<T>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let dimension = frontend.lock().unwrap().get_terminal_dimension()?;
        let screen = Screen {
            context: Context::new(),
            buffers: Vec::new(),
            receiver,
            lsp_manager: LspManager::new(sender.clone(), working_directory.clone()),
            sender,
            diagnostics: HashMap::new(),
            quickfix_lists: Rc::new(RefCell::new(QuickfixLists::new())),
            working_directory,
            layout: Layout::new(dimension),
            frontend,
        };
        Ok(screen)
    }

    pub fn run(
        &mut self,
        entry_path: Option<CanonicalizedPath>,
        event_receiver: Receiver<Event>,
    ) -> Result<(), anyhow::Error> {
        {
            let mut frontend = self.frontend.lock().unwrap();
            frontend.enter_alternate_screen()?;
            frontend.enable_raw_mode()?;
            frontend.enable_mouse_capture()?;
        }

        if let Some(entry_path) = entry_path {
            self.open_file(&entry_path, true)?;
        } else {
            self.open_file_picker()?;
        }

        self.render()?;

        let sender = self.sender.clone();
        std::thread::spawn(move || loop {
            if let Ok(event) = event_receiver.recv() {
                sender
                    .send(ScreenMessage::Event(event))
                    .unwrap_or_else(|e| {
                        log::error!("Failed to send event to screen: {}", e.to_string());
                    })
            }
        });

        while let Ok(message) = self.receiver.recv() {
            let should_quit = match message {
                ScreenMessage::Event(event) => self.handle_event(event),
                ScreenMessage::LspNotification(notification) => {
                    self.handle_lsp_notification(notification).map(|_| false)
                }
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

        // TODO: this line is a hack
        std::process::exit(0);

        Ok(())
    }

    fn components(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        self.layout.components()
    }

    /// Returns true if the screen should quit.
    fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        // Pass event to focused window
        let component = self.current_component();
        match event {
            Event::Key(key!("enter")) if self.context.mode.is_some() => {
                self.context.mode = None;
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
                    let dispatches = component
                        .borrow_mut()
                        .handle_event(&mut self.context, event);
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
        let grid = Grid::new(self.layout.terminal_dimension());

        // Render every window
        let (grid, cursor_point) = self
            .components()
            .into_iter()
            .map(|component| {
                let component = component.borrow();

                let rectangle = component
                    .rectangle()
                    .clamp_top(GLOBAL_TITLE_BAR_HEIGHT as usize);

                let path = component.editor().buffer().path();
                let diagnostics = path
                    .and_then(|path| self.diagnostics.get(&path))
                    .map(|diagnostics| diagnostics.as_slice())
                    .unwrap_or(&[]);

                let GetGridResult {
                    grid,
                    cursor_position,
                } = component.get_grid(&self.context.theme, diagnostics);
                let focused_component_id = self.layout.focused_component_id();
                let cursor_position = if focused_component_id
                    .map(|focused_component_id| component.id() == focused_component_id)
                    .unwrap_or(false)
                {
                    if let Some(cursor_position) = cursor_position {
                        // If cursor position is not in view
                        if cursor_position.line >= (rectangle.dimension().height) as usize {
                            None
                        } else {
                            Some(Position::new(
                                cursor_position.line + rectangle.origin.line,
                                cursor_position.column + rectangle.origin.column,
                            ))
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
                            self.context.theme.ui.window_title,
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
        let grid = {
            let mode = self
                .context
                .mode
                .as_ref()
                .map(|mode| mode.display())
                .or_else(|| {
                    self.current_component()
                        .map(|component| component.borrow().editor().display_mode())
                });

            let mode = if let Some(mode) = mode {
                format!("[{}]", mode)
            } else {
                String::new()
            };

            let title = format!("{} {}", self.working_directory.display(), mode);
            grid.set_line(0, &title, self.context.theme.ui.global_title)
        };

        self.render_grid(grid, cursor_point)?;

        Ok(())
    }

    fn render_grid(
        &mut self,
        grid: Grid,
        cursor_position: Option<Position>,
    ) -> Result<(), anyhow::Error> {
        let mut frontend = self.frontend.lock().unwrap();
        frontend.hide_cursor()?;
        frontend.render_grid(grid)?;
        if let Some(position) = cursor_position {
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

    fn handle_dispatch(&mut self, dispatch: Dispatch) -> Result<(), anyhow::Error> {
        match dispatch {
            Dispatch::CloseCurrentWindow { change_focused_to } => {
                self.close_current_window(change_focused_to)
            }
            Dispatch::SetSearch(search) => self.set_search(search),
            Dispatch::OpenSearchPrompt(search_kind) => self.open_search_prompt(search_kind),
            Dispatch::OpenFile { path } => {
                self.open_file(&path, true)?;
            }

            Dispatch::OpenFilePicker => {
                self.open_file_picker()?;
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
            Dispatch::DocumentDidChange { path, content } => {
                self.lsp_manager.document_did_change(path, content)?;
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
                if let Some(component) = self.current_component() {
                    let dispatches = component
                        .borrow_mut()
                        .editor_mut()
                        .apply_dispatch(&mut self.context, dispatch_editor)?;

                    self.handle_dispatches(dispatches)?;
                }
            }
            Dispatch::GotoLocation(location) => self.go_to_location(&location)?,
            Dispatch::OpenGlobalSearchPrompt(search_kind) => {
                self.open_global_search_prompt(search_kind)
            }
            Dispatch::GlobalSearch(search) => self.global_search(search)?,
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

    fn open_rename_prompt(&mut self, params: RequestParams) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Rename".to_string(),
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
        let working_directory = self.working_directory.clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Global Search".to_string(),
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
            history: self
                .context
                .previous_searches()
                .into_iter()
                .map(|search| search.search)
                .collect_vec(),
            owner: current_component.clone(),
            on_enter: Box::new(move |text, owner| {
                let search = Search {
                    kind,
                    search: text.to_string(),
                };
                if let Some(owner) = owner {
                    owner
                        .borrow_mut()
                        .editor_mut()
                        .select_match(Direction::Right, &Some(search.clone()))?;
                }
                Ok(vec![Dispatch::SetSearch(search)])
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
                        .map(|word| CompletionItem {
                            label: word,
                            documentation: None,
                            sort_text: None,
                            edit: None,
                        })
                        .collect_vec()
                })
                .unwrap_or_default(),
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
                    .map(|symbol| CompletionItem {
                        label: format!("{} ({:?})", symbol.name, symbol.kind),
                        documentation: None,
                        sort_text: None,
                        edit: None,
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

    fn open_file_picker(&mut self) -> anyhow::Result<()> {
        let current_component = self.current_component().clone();

        let working_directory = self.working_directory.clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Open File".to_string(),
            history: vec![],
            owner: current_component,
            on_enter: Box::new(move |current_item, _| {
                let path = working_directory.join(current_item)?;
                Ok(vec![Dispatch::OpenFile { path }])
            }),
            on_text_change: Box::new(|_, _| Ok(vec![])),
            items: {
                use git2::{Repository, StatusOptions};

                let git_status_files = {
                    let repo = Repository::open(&self.working_directory)?;
                    let mut opts = StatusOptions::new();
                    opts.include_untracked(true);
                    opts.include_ignored(false);
                    let statuses = repo.statuses(Some(&mut opts))?;
                    statuses
                        .iter()
                        .filter(|entry| !entry.status().is_ignored())
                        .filter_map(|entry| entry.path().map(|path| path.to_owned()))
                        .filter_map(|path| {
                            Some(CompletionItem {
                                label: CanonicalizedPath::try_from(&path)
                                    .ok()?
                                    .display_relative()
                                    .ok()?,
                                documentation: None,
                                sort_text: None,
                                edit: None,
                            })
                        })
                        .collect::<Vec<_>>()
                };

                let git_files = {
                    let repo = git2::Repository::open(&self.working_directory)?;

                    // Get the current branch name
                    let head = repo.head()?.target().map(Ok).unwrap_or_else(|| {
                        Err(anyhow!(
                            "Couldn't find HEAD for repository {}",
                            repo.path().display(),
                        ))
                    })?;

                    // Get the generic object of the current branch
                    let object = repo.find_object(head, None)?;

                    // Get the tree object of the current branch
                    let tree = object.peel_to_tree()?;

                    let mut result = vec![];
                    // Iterate over the tree entries and print their names
                    tree.walk(git2::TreeWalkMode::PostOrder, |root, entry| {
                        let entry_name = entry.name().unwrap_or_default();
                        let name = Path::new(root).join(entry_name);
                        let name = name.to_string_lossy();
                        result.push(name.to_string());
                        git2::TreeWalkResult::Ok
                    })?;

                    result
                        .into_iter()
                        .map(|word| CompletionItem {
                            label: word,
                            documentation: None,
                            sort_text: None,
                            edit: None,
                        })
                        .collect_vec()
                };
                git_files
                    .into_iter()
                    .chain(git_status_files)
                    .unique_by(|item| item.label.clone())
                    .collect_vec()
            },
        });
        self.layout
            .add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
        Ok(())
    }

    fn open_file(
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

        let buffer = Rc::new(RefCell::new(Buffer::from_path(entry_path)?));
        self.buffers.push(buffer.clone());
        let component = Rc::new(RefCell::new(SuggestiveEditor::from_buffer(
            buffer,
            SuggestiveEditorFilter::CurrentWord,
        )));

        if focus_editor {
            self.layout
                .add_and_focus_suggestive_editor(component.clone());
        } else {
            self.layout.add_suggestive_editor(component.clone());
        }

        self.update_component_diagnotics(
            entry_path,
            self.diagnostics
                .get(entry_path)
                .cloned()
                .unwrap_or_default(),
        );

        self.lsp_manager.open_file(entry_path.clone())?;

        Ok(component)
    }

    fn get_suggestive_editor(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<Rc<RefCell<SuggestiveEditor>>> {
        self.layout.get_suggestive_editor(component_id)
    }

    fn handle_lsp_notification(&mut self, notification: LspNotification) -> anyhow::Result<()> {
        match notification {
            LspNotification::Hover(component_id, hover) => {
                self.get_suggestive_editor(component_id)?
                    .borrow_mut()
                    .show_info("Hover info", hover.contents.join("\n\n"));
                Ok(())
            }
            LspNotification::Definition(_component_id, response) => {
                match response {
                    GotoDefinitionResponse::Single(location) => self.go_to_location(&location)?,
                    GotoDefinitionResponse::Multiple(locations) => {
                        if locations.is_empty() {
                            self.show_info(
                                "Goto definition info",
                                vec!["No definitions found".to_string()],
                            );
                        } else {
                            self.set_quickfix_list(QuickfixList::new(
                                locations.into_iter().map(QuickfixListItem::from).collect(),
                            ))?
                        }
                    }
                }

                Ok(())
            }
            LspNotification::References(_component_id, locations) => self.set_quickfix_list(
                QuickfixList::new(locations.into_iter().map(QuickfixListItem::from).collect()),
            ),
            LspNotification::Completion(component_id, completion) => {
                self.get_suggestive_editor(component_id)?
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
                log::info!("Received diagnostics");
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
            LspNotification::PrepareRenameResponse(component_id, _response) => {
                let editor = self.get_suggestive_editor(component_id)?;

                // Note: we cannot refactor the following code into the below code, otherwise we will get error,
                // because RefCell is borrow_mut twice. The borrow has to be dropped.
                //
                //
                //     if let Some(params) = editor.borrow().editor().get_request_params() {
                //         self.open_rename_prompt(params);
                //     }
                //
                let params = editor.borrow().editor().get_request_params();
                if let Some(params) = params {
                    self.open_rename_prompt(params);
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
            LspNotification::CodeAction(component_id, code_actions) => {
                let editor = self.get_suggestive_editor(component_id)?;
                editor.borrow_mut().set_code_actions(code_actions);
                Ok(())
            }
            LspNotification::SignatureHelp(component_id, signature_help) => {
                let editor = self.get_suggestive_editor(component_id)?;
                editor.borrow_mut().show_signature_help(signature_help);
                Ok(())
            }
            LspNotification::Symbols(component_id, symbols) => {
                self.open_symbol_picker(component_id, symbols)
            }
        }
    }

    fn update_diagnostics(&mut self, path: CanonicalizedPath, diagnostics: Vec<Diagnostic>) {
        self.update_component_diagnotics(&path, diagnostics.clone());
        self.diagnostics.insert(path, diagnostics);
    }

    fn update_component_diagnotics(&self, path: &CanonicalizedPath, diagnostics: Vec<Diagnostic>) {
        let component = self
            .components()
            .iter()
            .find(|component| {
                component
                    .borrow()
                    .editor()
                    .buffer()
                    .path()
                    .map(|buffer_path| &buffer_path == path)
                    .unwrap_or(false)
            })
            .cloned();

        if let Some(component) = component {
            component
                .borrow_mut()
                .editor_mut()
                .set_diagnostics(diagnostics);
        }
    }

    fn goto_quickfix_list_item(&mut self, direction: Direction) -> anyhow::Result<()> {
        let item = self.quickfix_lists.borrow_mut().get_item(direction);
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
            QuickfixListType::LspDiagnostic => {
                let quickfix_list = QuickfixList::new(
                    self.diagnostics
                        .iter()
                        .flat_map(|(path, diagnostics)| {
                            diagnostics.iter().map(|diagnostic| {
                                QuickfixListItem::new(
                                    Location {
                                        path: path.clone(),
                                        range: diagnostic.range.clone(),
                                    },
                                    vec![diagnostic.message()],
                                )
                            })
                        })
                        .collect(),
                );

                self.set_quickfix_list(quickfix_list)
            }
            QuickfixListType::Items(items) => self.set_quickfix_list(QuickfixList::new(items)),
        }
    }

    fn set_quickfix_list(&mut self, quickfix_list: QuickfixList) -> anyhow::Result<()> {
        self.context.mode = Some(GlobalMode::QuickfixListItem);
        self.quickfix_lists.borrow_mut().push(quickfix_list);
        self.layout.show_quickfix_lists(self.quickfix_lists.clone());
        self.goto_quickfix_list_item(Direction::Right)
    }

    fn apply_workspace_edit(&mut self, workspace_edit: WorkspaceEdit) -> Result<(), anyhow::Error> {
        for edit in workspace_edit.edits {
            let component = self.open_file(&edit.path, false)?;
            let dispatches = component
                .borrow_mut()
                .editor_mut()
                .apply_positional_edits(edit.edits);

            self.handle_dispatches(dispatches)?;

            let dispatches = component.borrow_mut().editor_mut().save()?;

            self.handle_dispatches(dispatches)?;
        }
        Ok(())
    }

    fn show_keymap_legend(&mut self, keymap_legend_config: KeymapLegendConfig) {
        self.layout.show_keymap_legend(keymap_legend_config)
    }

    fn global_search(&mut self, search: Search) -> anyhow::Result<()> {
        let working_directory = self.working_directory.clone();
        let locations = match search.kind {
            SearchKind::Regex => {
                list::grep::run(&search.search, working_directory.clone().into(), false)
            }
            SearchKind::Literal => {
                list::grep::run(&search.search, working_directory.clone().into(), true)
            }
            SearchKind::AstGrep => {
                todo!()
            }
        }?;
        self.set_quickfix_list(QuickfixList::new(
            locations
                .into_iter()
                .map(|location| QuickfixListItem::new(location, vec![]))
                .collect_vec(),
        ))
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
    OpenFilePicker,
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
        path: CanonicalizedPath,
        content: String,
    },
    DocumentDidSave {
        path: CanonicalizedPath,
    },
    SetQuickfixList(QuickfixListType),
    GotoQuickfixListItem(Direction),
    GotoOpenedEditor(Direction),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestParams {
    pub component_id: ComponentId,
    pub path: CanonicalizedPath,
    pub position: Position,
}

#[derive(Debug)]
pub enum ScreenMessage {
    LspNotification(LspNotification),
    Event(Event),
}
