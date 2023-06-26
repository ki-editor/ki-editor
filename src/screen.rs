use anyhow::anyhow;
use itertools::Itertools;
use std::{
    cell::RefCell,
    collections::HashMap,
    io::stdout,
    path::Path,
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
};

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use tree_sitter::Point;

use crate::{
    buffer::Buffer,
    canonicalized_path::CanonicalizedPath,
    components::{
        component::{Component, ComponentId},
        editor::Direction,
        prompt::{Prompt, PromptConfig},
        suggestive_editor::{SuggestiveEditor, SuggestiveEditorFilter},
    },
    context::Context,
    grid::{Grid, Style},
    layout::Layout,
    lsp::{
        completion::CompletionItem, diagnostic::Diagnostic,
        goto_definition_response::GotoDefinitionResponse, manager::LspManager,
        process::LspNotification, workspace_edit::WorkspaceEdit,
    },
    position::Position,
    quickfix_list::{Location, QuickfixList, QuickfixListItem, QuickfixListType, QuickfixLists},
};

pub struct Screen {
    context: Context,

    /// Used for diffing to reduce unnecessary re-painting.
    previous_grid: Option<Grid>,

    buffers: Vec<Rc<RefCell<Buffer>>>,

    sender: Sender<ScreenMessage>,

    /// Used for receiving message from various sources:
    /// - Events from crossterm
    /// - Notifications from language server
    receiver: Receiver<ScreenMessage>,

    lsp_manager: LspManager,

    diagnostics: HashMap<CanonicalizedPath, Vec<Diagnostic>>,

    quickfix_lists: Rc<RefCell<QuickfixLists>>,

    layout: Layout,
}

impl Screen {
    pub fn new() -> anyhow::Result<Screen> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let (width, height) = terminal::size()?;
        let screen = Screen {
            context: Context::new(),
            previous_grid: None,
            buffers: Vec::new(),
            receiver,
            lsp_manager: LspManager::new(sender.clone()),
            sender,
            diagnostics: HashMap::new(),
            quickfix_lists: Rc::new(RefCell::new(QuickfixLists::new())),
            layout: Layout::new(Dimension { height, width }),
        };
        Ok(screen)
    }

    pub fn run(&mut self, entry_path: &CanonicalizedPath) -> Result<(), anyhow::Error> {
        self.open_file(entry_path, true)?;

        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;
        stdout.execute(EnableMouseCapture)?;

        self.render(&mut stdout)?;

        let sender = self.sender.clone();
        std::thread::spawn(move || loop {
            if let Ok(event) = crossterm::event::read() {
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
                self.show_info(vec![e.to_string()]);
                log::error!("{:?}", e);
                false
            });

            if should_quit {
                break;
            }

            self.render(&mut stdout)?;
        }

        stdout.execute(LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;

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
            Event::Key(event) => match event.code {
                KeyCode::Char('%') => {
                    // let cloned = component.clone();
                    // self.focused_component_id = self.add_component(cloned);
                }
                KeyCode::Char('f') if event.modifiers == KeyModifiers::CONTROL => {
                    self.open_search_prompt()
                }
                KeyCode::Char('o') if event.modifiers == KeyModifiers::CONTROL => {
                    self.open_file_picker()?;
                }
                KeyCode::Char('q') if event.modifiers == KeyModifiers::CONTROL => {
                    if self.quit() {
                        return Ok(true);
                    }
                }
                KeyCode::Char('w') if event.modifiers == KeyModifiers::CONTROL => {
                    self.layout.change_view()
                }
                _ => {
                    component.map(|component| {
                        let dispatches = component
                            .borrow_mut()
                            .handle_event(&mut self.context, Event::Key(event));
                        self.handle_dispatches_result(dispatches)
                    });
                }
            },
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
                    self.handle_dispatches_result(dispatches);
                });
            }
        }
        Ok(false)
    }

    /// Return true if there's no more windows
    fn quit(&mut self) -> bool {
        self.layout.remove_current_component()
    }

    fn render(&mut self, stdout: &mut std::io::Stdout) -> Result<(), anyhow::Error> {
        // Recalculate layout before each render
        self.layout.recalculate_layout();

        // Generate layout
        let grid = Grid::new(self.layout.terminal_dimension());

        // Render every window
        let (grid, cursor_point) = self
            .components()
            .into_iter()
            .map(|component| {
                let component = component.borrow();

                let rectangle = component.rectangle();

                let path = component.editor().buffer().path();
                let diagnostics = path
                    .and_then(|path| self.diagnostics.get(&path))
                    .map(|diagnostics| diagnostics.as_slice())
                    .unwrap_or(&[]);

                let component_grid = component.get_grid(diagnostics);
                let focused_component_id = self.layout.focused_component_id();
                let cursor_point = if focused_component_id
                    .map(|focused_component_id| component.id() == focused_component_id)
                    .unwrap_or(false)
                {
                    let cursor_position = component.get_cursor_position();
                    let scroll_offset = component.scroll_offset();

                    // If cursor position is not in view
                    if cursor_position.line < scroll_offset as usize
                        || cursor_position.line
                            >= (scroll_offset + rectangle.dimension().height) as usize
                    {
                        None
                    } else {
                        Some(Point::new(
                            (cursor_position.line + rectangle.origin.row)
                                .saturating_sub(scroll_offset as usize),
                            cursor_position.column + rectangle.origin.column,
                        ))
                    }
                } else {
                    None
                };

                (
                    component_grid,
                    rectangle.clone(),
                    cursor_point,
                    component.title(),
                )
            })
            .fold(
                (grid, None),
                |(grid, current_cursor_point), (component_grid, rectangle, cursor_point, title)| {
                    {
                        let title_rectangle = rectangle.move_up(1).set_height(1);
                        let title_grid = Grid::new(title_rectangle.dimension()).set_line(
                            0,
                            &title,
                            Style {
                                foreground_color: Color::White,
                                background_color: Color::DarkGrey,
                            },
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
            .iter()
            .fold(grid, |grid, border| grid.set_border(border));

        self.render_grid(grid, cursor_point, stdout)?;

        Ok(())
    }

    fn render_grid(
        &mut self,
        grid: Grid,
        cursor_point: Option<Point>,
        stdout: &mut std::io::Stdout,
    ) -> Result<(), anyhow::Error> {
        queue!(stdout, Hide)?;
        let cells = {
            let diff = if let Some(previous_grid) = self.previous_grid.take() {
                previous_grid.diff(&grid)
            } else {
                queue!(stdout, Clear(ClearType::All)).unwrap();
                grid.to_position_cells()
            };

            self.previous_grid = Some(grid);

            diff
        };

        for cell in cells.into_iter() {
            queue!(
                stdout,
                MoveTo(cell.position.column as u16, cell.position.line as u16)
            )?;
            queue!(
                stdout,
                SetBackgroundColor(cell.cell.background_color),
                SetForegroundColor(cell.cell.foreground_color),
                Print(reveal(cell.cell.symbol))
            )?;
        }

        if let Some(point) = cursor_point {
            queue!(stdout, Show)?;
            queue!(stdout, SetCursorStyle::BlinkingBlock)?;
            execute!(stdout, MoveTo(point.column as u16, point.row as u16))?;
        }

        Ok(())
    }

    fn handle_dispatches_result(&mut self, dispatches: anyhow::Result<Vec<Dispatch>>) {
        match dispatches {
            Ok(dispatches) => {
                self.handle_dispatches(dispatches).unwrap_or_else(|error| {
                    // todo!("Show the error to the user")
                    log::error!("Error: {:?}", error);
                });
            }
            Err(error) => {
                // todo!("Show the error to the user")
                log::error!("Error: {:?}", error);
            }
        }
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
            Dispatch::SetSearch { search } => self.set_search(search),
            Dispatch::OpenFile { path } => {
                self.open_file(&path, true)?;
            }
            Dispatch::RequestCompletion(params) => {
                self.lsp_manager.request_completion(params)?;
            }
            Dispatch::RequestReferences(params) => self.lsp_manager.request_references(params)?,
            Dispatch::RequestHover(params) => {
                self.lsp_manager.request_hover(params)?;
            }
            Dispatch::RequestDefinition(params) => {
                self.lsp_manager.request_definition(params)?;
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
            Dispatch::DocumentDidChange { path, content } => {
                self.lsp_manager.document_did_change(path, content)?;
            }
            Dispatch::DocumentDidSave { path } => {
                self.lsp_manager.document_did_save(path)?;
            }
            Dispatch::ShowInfo { content } => self.show_info(content),
            Dispatch::SetQuickfixList(r#type) => self.set_quickfix_list_type(r#type)?,
            Dispatch::GotoQuickfixListItem(direction) => self.goto_quickfix_list_item(direction)?,
            Dispatch::GotoOpenedEditor(direction) => self.layout.goto_opened_editor(direction),
            Dispatch::ApplyWorkspaceEdit(workspace_edit) => {
                log::info!("Applying workspace edit: {:#?}", workspace_edit);
                self.apply_workspace_edit(workspace_edit)?;
            }
        }
        Ok(())
    }

    fn current_component(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.layout.current_component()
    }

    fn close_current_window(&mut self, change_focused_to: ComponentId) {
        self.layout.close_current_window(change_focused_to)
    }

    fn set_search(&mut self, search: String) {
        self.context.set_search(search);
    }

    fn resize(&mut self, dimension: Dimension) {
        // Remove the previous_grid so that the entire screen is re-rendered
        // Because diffing when the size has change is not supported yet.
        self.previous_grid.take();
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

    fn open_search_prompt(&mut self) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Search".to_string(),
            history: self.context.previous_searches(),
            owner: current_component.clone(),
            on_enter: Box::new(|text, owner| {
                owner
                    .map(|owner| {
                        owner
                            .borrow_mut()
                            .editor_mut()
                            .select_match(Direction::Forward, &Some(text.to_string()));
                    })
                    .unwrap_or_default();
                Ok(vec![Dispatch::SetSearch {
                    search: text.to_string(),
                }])
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

    fn open_file_picker(&mut self) -> anyhow::Result<()> {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Open File".to_string(),
            history: vec![],
            owner: current_component,
            on_enter: Box::new(|current_item, _| {
                Ok(vec![Dispatch::OpenFile {
                    path: CanonicalizedPath::try_from(current_item)?,
                }])
            }),
            on_text_change: Box::new(|_, _| Ok(vec![])),
            items: {
                let repo = git2::Repository::open(".")?;

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
                    .show_info(hover.contents.join("\n\n"));
                Ok(())
            }
            LspNotification::Definition(_component_id, response) => {
                match response {
                    GotoDefinitionResponse::Single(location) => self.go_to_location(&location)?,
                    GotoDefinitionResponse::Multiple(locations) => {
                        self.set_quickfix_list(QuickfixList::new(
                            locations.into_iter().map(QuickfixListItem::from).collect(),
                        ))?
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
                self.update_diagnostics(
                    params
                        .uri
                        .to_file_path()
                        .map_err(|err| {
                            anyhow::anyhow!("Couldn't convert URI to file path: {:?}", err)
                        })?
                        .try_into()?,
                    params
                        .diagnostics
                        .into_iter()
                        .map(Diagnostic::from)
                        .collect::<Vec<_>>(),
                );
                Ok(())
            }
            LspNotification::PrepareRenameResponse(component_id, _response) => {
                let editor = self.get_suggestive_editor(component_id)?;

                let params = editor.borrow().editor().get_request_params();

                // Note: we cannot refactor the following code into the below code, otherwise we will get error,
                // because RefCell is borrow_mut twice. The borrow has to be dropped.
                //
                //
                // if let Some(params) = editor.borrow().editor().get_request_params() {
                //   self.open_rename_prompt(params);
                // }
                if let Some(params) = params {
                    self.open_rename_prompt(params);
                }

                Ok(())
            }
            LspNotification::Error(error) => {
                self.show_info(vec![error]);
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

    fn show_info(&mut self, contents: Vec<String>) {
        self.layout.show_info(contents)
    }

    fn go_to_location(&mut self, location: &Location) -> Result<(), anyhow::Error> {
        let component = self.open_file(&location.path, true)?;
        component
            .borrow_mut()
            .editor_mut()
            .set_selection(location.range.clone());
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
        }
    }

    fn set_quickfix_list(&mut self, quickfix_list: QuickfixList) -> anyhow::Result<()> {
        self.quickfix_lists.borrow_mut().push(quickfix_list);
        self.layout.show_quickfix_lists(self.quickfix_lists.clone());
        self.goto_quickfix_list_item(Direction::Forward)
    }

    fn apply_workspace_edit(&mut self, workspace_edit: WorkspaceEdit) -> Result<(), anyhow::Error> {
        for edit in workspace_edit.edits {
            let component = self.open_file(&edit.path, false)?;
            let dispatches = component
                .borrow_mut()
                .editor_mut()
                .apply_positional_edits(edit.edits);

            self.handle_dispatches(dispatches)?;

            component.borrow_mut().editor_mut().save()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Dimension {
    pub height: u16,
    pub width: u16,
}

/// Convert invisible character to visible character
fn reveal(s: String) -> String {
    match s.as_str() {
        "\n" => " ".to_string(),
        _ => s,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Dispatch are for child component to request action from the root node
pub enum Dispatch {
    CloseCurrentWindow {
        change_focused_to: ComponentId,
    },
    SetSearch {
        search: String,
    },
    OpenFile {
        path: CanonicalizedPath,
    },
    ShowInfo {
        content: Vec<String>,
    },
    RequestCompletion(RequestParams),
    RequestHover(RequestParams),
    RequestDefinition(RequestParams),
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
