use anyhow::anyhow;
use itertools::Itertools;
use std::{
    cell::RefCell,
    collections::HashMap,
    io::stdout,
    path::{Path, PathBuf},
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
    components::{
        component::{Component, ComponentId},
        editor::Direction,
        editor::Editor,
        prompt::{Prompt, PromptConfig},
        suggestive_editor::SuggestiveEditor,
    },
    diagnostic::Diagnostic,
    grid::{Grid, Style},
    lsp::{manager::LspManager, process::LspNotification},
    position::Position,
    quickfix_list::{Location, QuickfixList, QuickfixListItem, QuickfixListType, QuickfixLists},
    rectangle::{Border, Rectangle},
};

pub struct Screen {
    focused_component_id: ComponentId,

    state: State,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    /// Used for diffing to reduce unnecessary re-painting.
    previous_grid: Option<Grid>,

    buffers: Vec<Rc<RefCell<Buffer>>>,

    sender: Sender<ScreenMessage>,

    /// Used for receiving message from various sources:
    /// - Events from crossterm
    /// - Notifications from language server
    receiver: Receiver<ScreenMessage>,

    lsp_manager: LspManager,

    /// Saved for populating completions
    suggestive_editors: Vec<Rc<RefCell<SuggestiveEditor>>>,

    diagnostics: HashMap<PathBuf, Vec<Diagnostic>>,

    quickfix_lists: QuickfixLists,

    /// The layout of the app is split into 3 section: the main panel, info panel and prompts.
    /// The main panel is where the user edits code, and the info panel is for displaying info like
    /// hover text, diagnostics, etc.
    main_panel: Option<Rc<RefCell<SuggestiveEditor>>>,
    info_panel: Option<Rc<RefCell<Editor>>>,
    prompts: Vec<Rc<RefCell<Prompt>>>,
}

pub struct State {
    terminal_dimension: Dimension,
    previous_searches: Vec<String>,
}
impl State {
    pub fn last_search(&self) -> Option<String> {
        self.previous_searches.last().map(|s| s.clone())
    }
}

impl Screen {
    pub fn new() -> anyhow::Result<Screen> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let (width, height) = terminal::size()?;
        let dimension = Dimension { height, width };
        let (rectangles, borders) = Rectangle::generate(1, dimension);
        let screen = Screen {
            state: State {
                terminal_dimension: dimension,
                previous_searches: vec![],
            },
            focused_component_id: ComponentId::new(),
            rectangles,
            borders,
            previous_grid: None,
            buffers: Vec::new(),
            receiver,
            lsp_manager: LspManager::new(sender.clone()),
            sender,
            suggestive_editors: Vec::new(),
            diagnostics: HashMap::new(),
            quickfix_lists: QuickfixLists::new(),
            main_panel: None,
            info_panel: None,
            prompts: Vec::new(),
        };
        Ok(screen)
    }

    pub fn run(&mut self, entry_path: &PathBuf) -> Result<(), anyhow::Error> {
        self.open_file(entry_path)?;

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
        let root_components = vec![]
            .into_iter()
            .chain(
                self.main_panel
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.prompts
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.info_panel
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .collect::<Vec<_>>();

        let mut components = root_components.clone();
        for component in root_components.iter() {
            components.extend(component.borrow().children());
        }
        components
    }

    fn get_component(&self, id: ComponentId) -> Rc<RefCell<dyn Component>> {
        self.components()
            .into_iter()
            .find(|c| c.borrow().id() == id)
            .unwrap()
    }

    /// Returns true if the screen should quit.
    fn handle_event(&mut self, event: Event) -> anyhow::Result<bool> {
        // Pass event to focused window
        let component = self.get_component(self.focused_component_id);
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
                    self.open_file_picker()
                }
                KeyCode::Char('q') if event.modifiers == KeyModifiers::CONTROL => {
                    if self.quit() {
                        return Ok(true);
                    }
                }
                KeyCode::Char('w') if event.modifiers == KeyModifiers::CONTROL => {
                    self.change_view()
                }
                _ => {
                    let dispatches = component
                        .borrow_mut()
                        .handle_event(&self.state, Event::Key(event));
                    self.handle_dispatches_result(dispatches)
                }
            },
            Event::Resize(columns, rows) => {
                self.resize(Dimension {
                    height: rows,
                    width: columns,
                });
            }
            event => {
                let dispatches = component.borrow_mut().handle_event(&self.state, event);
                self.handle_dispatches_result(dispatches);
            }
        }
        Ok(false)
    }

    fn remove_current_component(&mut self) {
        self.suggestive_editors = self
            .suggestive_editors
            .iter()
            .filter(|c| c.borrow().id() != self.focused_component_id)
            .cloned()
            .collect();

        self.prompts = self
            .prompts
            .iter()
            .filter(|c| c.borrow().id() != self.focused_component_id)
            .cloned()
            .collect();

        self.main_panel = self
            .main_panel
            .take()
            .filter(|c| c.borrow().id() != self.focused_component_id);

        self.info_panel = self
            .info_panel
            .take()
            .filter(|c| c.borrow().id() != self.focused_component_id);

        self.set_main_panel(self.suggestive_editors.last().cloned());
    }

    // Return true if there's no more windows
    fn quit(&mut self) -> bool {
        // Remove current component
        self.remove_current_component();

        if let Some(component) = self.components().last() {
            self.focused_component_id = component.borrow().id();
            self.recalculate_layout();
            false
        } else {
            true
        }
    }

    fn add_and_focus_prompt(&mut self, prompt: Rc<RefCell<Prompt>>) {
        self.focused_component_id = prompt.borrow().id();
        self.prompts.push(prompt);
        self.recalculate_layout();
    }

    fn render(&mut self, stdout: &mut std::io::Stdout) -> Result<(), anyhow::Error> {
        // Recalculate layout before each render
        self.recalculate_layout();

        // Generate layout
        let grid = Grid::new(self.state.terminal_dimension);

        // Render every window
        let (grid, cursor_point) = self
            .components()
            .into_iter()
            .map(|component| {
                let component = component.borrow();

                let rectangle = component.rectangle();

                let path = component.editor().buffer().path();
                let diagnostics = path
                    .map(|path| self.diagnostics.get(&path))
                    .flatten()
                    .map(|diagnostics| diagnostics.as_slice())
                    .unwrap_or(&[]);

                let component_grid = component.get_grid(diagnostics);
                let cursor_point = if component.id() == self.focused_component_id {
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
                    component.title().to_string(),
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
                            current_cursor_point.or_else(|| cursor_point),
                        )
                    }
                },
            );

        // Render every border
        let grid = self
            .borders
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

            self.previous_grid = Some(grid.clone());

            diff
        };

        for cell in cells.into_iter() {
            queue!(
                stdout,
                MoveTo(cell.position.column as u16, cell.position.row as u16)
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
                self.open_file(&path)?;
            }
            Dispatch::RequestCompletion {
                component_id,
                path,
                position,
            } => {
                self.lsp_manager
                    .request_completion(component_id, path, position)?;
            }
            Dispatch::DocumentDidChange { path, content } => {
                self.lsp_manager.document_did_change(path, content)?;
            }
            Dispatch::DocumentDidSave { path } => {
                self.lsp_manager.document_did_save(path)?;
            }
            Dispatch::RequestHover {
                component_id,
                path,
                position,
            } => {
                self.lsp_manager
                    .request_hover(component_id, path, position)?;
            }
            Dispatch::ShowInfo { content } => self.show_info(content),
            Dispatch::SetQuickfixList(r#type) => match r#type {
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
                                        vec![diagnostic.message.clone()],
                                    )
                                })
                            })
                            .collect(),
                    );

                    self.quickfix_lists.push(quickfix_list);
                }
            },
            Dispatch::NextQuickfixListItem => self.to_quickfix_list_item(Direction::Forward)?,
            Dispatch::PreviousQuickfixListItem => {
                self.to_quickfix_list_item(Direction::Backward)?
            }
        }
        Ok(())
    }

    fn current_component(&self) -> Rc<RefCell<dyn Component>> {
        self.get_component(self.focused_component_id)
    }

    fn close_current_window(&mut self, change_focused_to: ComponentId) {
        self.remove_current_component();
        self.focused_component_id = change_focused_to;
        self.recalculate_layout();
    }

    fn set_search(&mut self, search: String) {
        self.state.previous_searches.push(search)
    }

    fn resize(&mut self, dimension: Dimension) {
        // Remove the previous_grid so that the entire screen is re-rendered
        // Because diffing when the size has change is not supported yet.
        self.previous_grid.take();
        self.state.terminal_dimension = dimension;

        self.recalculate_layout()
    }

    fn recalculate_layout(&mut self) {
        let (rectangles, borders) =
            Rectangle::generate(self.components().len(), self.state.terminal_dimension);
        self.rectangles = rectangles;
        self.borders = borders;

        self.components()
            .into_iter()
            .zip(self.rectangles.iter())
            .for_each(|(component, rectangle)| {
                // Leave 1 row on top for rendering the title
                let (_, rectangle) = rectangle.split_vertically_at(1);
                component.borrow_mut().set_rectangle(rectangle.clone())
            });
    }

    fn open_search_prompt(&mut self) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Search".to_string(),
            history: self.state.previous_searches.clone(),
            owner: current_component,
            on_enter: Box::new(|text, _, owner| {
                owner
                    .borrow_mut()
                    .editor_mut()
                    .select_match(Direction::Forward, &Some(text.to_string()));
                vec![Dispatch::SetSearch {
                    search: text.to_string(),
                }]
            }),
            on_text_change: Box::new(|current_text, owner| {
                owner
                    .borrow_mut()
                    .editor_mut()
                    .select_match(Direction::Forward, &Some(current_text.to_string()));
                Ok(vec![])
            }),
            get_suggestions: Box::new(|text, owner| {
                Ok(owner.borrow().editor().buffer().find_words(&text))
            }),
        });
        self.add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn open_file_picker(&mut self) {
        let current_component = self.current_component().clone();
        let prompt = Prompt::new(PromptConfig {
            title: "Open File".to_string(),
            history: vec![],
            owner: current_component,
            on_enter: Box::new(|_, current_item, _| {
                vec![Dispatch::OpenFile {
                    path: Path::new(current_item).to_path_buf(),
                }]
            }),
            on_text_change: Box::new(|_, _| Ok(vec![])),
            get_suggestions: Box::new(|text, _| {
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
                    if name.to_lowercase().contains(&text.to_lowercase()) {
                        result.push(name.to_string());
                    }
                    git2::TreeWalkResult::Ok
                })?;
                Ok(result)
            }),
        });
        self.add_and_focus_prompt(Rc::new(RefCell::new(prompt)));
    }

    fn change_view(&mut self) {
        let components = self.components();
        if let Some(component) = components
            .iter()
            .find(|component| component.borrow().id() > self.focused_component_id)
            .map_or_else(
                || {
                    components
                        .iter()
                        .min_by(|x, y| x.borrow().id().cmp(&y.borrow().id()))
                },
                |component| Some(component),
            )
        {
            self.focused_component_id = component.borrow().id()
        }
    }

    fn open_file(&mut self, entry_path: &PathBuf) -> anyhow::Result<Rc<RefCell<dyn Component>>> {
        // Check if the file is opened before
        // so that we won't notify the LSP twice
        if let Some(matching_editor) = self.suggestive_editors.iter().cloned().find(|component| {
            component
                .borrow()
                .editor()
                .buffer()
                .path()
                .map(|path| &path == entry_path)
                .unwrap_or(false)
        }) {
            self.set_main_panel(Some(matching_editor.clone()));
            return Ok(matching_editor.clone());
        }

        let buffer = Rc::new(RefCell::new(Buffer::from_path(&entry_path)?));
        self.buffers.push(buffer.clone());
        let entry_component = Rc::new(RefCell::new(SuggestiveEditor::from_buffer(buffer)));

        self.suggestive_editors.push(entry_component.clone());

        log::info!("self.main_panel = {:?}", entry_component.borrow().id());
        self.set_main_panel(Some(entry_component.clone()));

        self.update_component_diagnotics(
            &entry_path,
            self.diagnostics
                .get(entry_path)
                .cloned()
                .unwrap_or_default(),
        );

        self.lsp_manager.open_file(entry_path.clone())?;

        Ok(entry_component)
    }

    fn get_suggestive_editor(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<Rc<RefCell<SuggestiveEditor>>> {
        self.suggestive_editors
            .iter()
            .find(|editor| editor.borrow().id() == component_id)
            .cloned()
            .ok_or_else(|| anyhow!("Couldn't find component with id {:?}", component_id))
    }

    fn handle_lsp_notification(&mut self, notification: LspNotification) -> anyhow::Result<()> {
        match notification {
            LspNotification::Hover(component_id, hover) => {
                fn marked_string_to_string(marked_string: lsp_types::MarkedString) -> String {
                    match marked_string {
                        lsp_types::MarkedString::String(string) => string,
                        lsp_types::MarkedString::LanguageString(language_string) => {
                            language_string.value
                        }
                    }
                }
                let content = match hover.contents {
                    lsp_types::HoverContents::Scalar(marked_string) => {
                        marked_string_to_string(marked_string)
                    }
                    lsp_types::HoverContents::Array(contents) => contents
                        .into_iter()
                        .map(marked_string_to_string)
                        .collect::<Vec<_>>()
                        .join("----------------\n\n"),
                    lsp_types::HoverContents::Markup(content) => content.value,
                };
                self.show_info(content);
                Ok(())
            }
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
                    params.uri.to_file_path().map_err(|err| {
                        anyhow::anyhow!("Couldn't convert URI to file path: {:?}", err)
                    })?,
                    params
                        .diagnostics
                        .into_iter()
                        .map(Diagnostic::from)
                        .collect::<Vec<_>>(),
                );
                Ok(())
            }
        }
    }

    fn update_diagnostics(&mut self, path: PathBuf, diagnostics: Vec<Diagnostic>) {
        self.update_component_diagnotics(&path, diagnostics.clone());
        self.diagnostics.insert(path, diagnostics);
    }

    fn update_component_diagnotics(&self, path: &PathBuf, diagnostics: Vec<Diagnostic>) {
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

    fn to_quickfix_list_item(&mut self, direction: Direction) -> anyhow::Result<()> {
        if let Some(item) = self
            .quickfix_lists
            .current_mut()
            .map(|quickfix_list| match direction {
                Direction::Current => quickfix_list.current_item(),
                Direction::Forward => quickfix_list.next_item(),
                Direction::Backward => quickfix_list.previous_item(),
            })
            .flatten()
            .cloned()
        {
            if let Some(info) = item.info() {
                self.show_info(info);
            }
            self.go_to_location(item.location())?;
        }
        Ok(())
    }

    fn show_info(&mut self, info: String) {
        match &self.info_panel {
            None => {
                let info_panel = Rc::new(RefCell::new(Editor::from_text(
                    tree_sitter_md::language(),
                    &info,
                )));
                self.info_panel = Some(info_panel.clone());
            }
            Some(info_panel) => info_panel.borrow_mut().update(&info),
        }
    }

    fn current_buffer_path(&self) -> Option<PathBuf> {
        self.current_component().borrow().editor().buffer().path()
    }

    fn set_main_panel(&mut self, editor: Option<Rc<RefCell<SuggestiveEditor>>>) {
        if let Some(editor) = &editor {
            self.focused_component_id = editor.borrow().id();
        }
        self.main_panel = editor;
    }

    fn to_editor(&mut self, direction: Direction) {
        let editor = self
            .suggestive_editors
            .iter()
            .find(|editor| {
                let id = editor.borrow().id();

                match direction {
                    Direction::Forward | Direction::Current => id > self.focused_component_id,
                    Direction::Backward => id < self.focused_component_id,
                }
            })
            .cloned()
            .or_else(|| self.main_panel.take());
        self.set_main_panel(editor);
    }

    fn go_to_location(&mut self, location: &Location) -> Result<(), anyhow::Error> {
        let component = self.open_file(&location.path)?;
        component
            .borrow_mut()
            .editor_mut()
            .set_selection(location.range.clone());
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

#[derive(Clone, Debug)]
/// Dispatch are for child component to request action from the root node
pub enum Dispatch {
    CloseCurrentWindow {
        change_focused_to: ComponentId,
    },
    SetSearch {
        search: String,
    },
    OpenFile {
        path: PathBuf,
    },
    ShowInfo {
        content: String,
    },
    RequestCompletion {
        component_id: ComponentId,
        path: PathBuf,
        position: Position,
    },
    RequestHover {
        component_id: ComponentId,
        path: PathBuf,
        position: Position,
    },
    DocumentDidChange {
        path: PathBuf,
        content: String,
    },
    DocumentDidSave {
        path: PathBuf,
    },
    SetQuickfixList(QuickfixListType),
    NextQuickfixListItem,
    PreviousQuickfixListItem,
}

#[derive(Debug)]
pub enum ScreenMessage {
    LspNotification(LspNotification),
    Event(Event),
}
