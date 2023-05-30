use std::{
    cell::RefCell,
    io::stdout,
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
};

use anyhow::anyhow;

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    ExecutableCommand,
};
use lsp_types::Position;
use tree_sitter::Point;

use crate::{
    buffer::Buffer,
    components::{
        component::{Component, ComponentId},
        dropdown::DropdownItem,
        editor::Direction,
        prompt::{Prompt, PromptConfig},
        suggestive_editor::SuggestiveEditor,
    },
    grid::{Grid, Style},
    lsp::{manager::LspManager, process::LspNotification},
    rectangle::{Border, Rectangle},
};

pub struct Screen {
    focused_component_id: ComponentId,

    root_components: Vec<Rc<RefCell<dyn Component>>>,

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
            root_components: Vec::new(),
            previous_grid: None,
            buffers: Vec::new(),
            receiver,
            lsp_manager: LspManager::new(sender.clone()),
            sender,
            suggestive_editors: Vec::new(),
        };
        Ok(screen)
    }

    pub fn run(&mut self, entry_path: PathBuf) -> Result<(), anyhow::Error> {
        crossterm::terminal::enable_raw_mode()?;

        self.open_file(entry_path)?;

        let mut stdout = stdout();

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
            match message {
                ScreenMessage::Event(event) => self.handle_event(event),
                ScreenMessage::LspNotification(notification) => {
                    self.handle_lsp_notification(notification)
                }
            }
            .unwrap_or_else(|e| log::error!("{:?}", e));

            self.render(&mut stdout)?;
        }

        Ok(())
    }

    fn components(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        let mut components = self.root_components.clone();
        for component in self.root_components.iter() {
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

    fn handle_event(&mut self, event: Event) -> anyhow::Result<()> {
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
                        // self.lsp_server_process.lock().unwrap().shutdown()?;
                        crossterm::terminal::disable_raw_mode()?;

                        // Quit the process
                        std::process::exit(0);
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
        Ok(())
    }

    fn remove_current_component(&mut self) {
        self.root_components = self
            .root_components
            .iter()
            .filter(|c| c.borrow().id() != self.focused_component_id)
            .cloned()
            .collect();
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

    fn add_and_focus_component(&mut self, entry_component: Rc<RefCell<dyn Component>>) {
        self.focused_component_id = entry_component.borrow().id();
        self.root_components.push(entry_component);
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
                let component_grid = component.get_grid();
                let cursor_point = if component.id() == self.focused_component_id {
                    let cursor_position = component.get_cursor_point();
                    let scroll_offset = component.scroll_offset();

                    // If cursor position is not in view
                    if cursor_position.row < scroll_offset as usize
                        || cursor_position.row
                            >= (scroll_offset + rectangle.dimension().height) as usize
                    {
                        None
                    } else {
                        Some(Point::new(
                            (cursor_position.row + rectangle.origin.row)
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
            queue!(stdout, MoveTo(point.column as u16, point.row as u16))?;
            queue!(stdout, MoveTo(point.column as u16, point.row as u16))?;
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
            Dispatch::OpenFile { path } => self.open_file(path)?,
            Dispatch::RequestCompletion { position } => {
                let current_path = self.current_component().borrow().editor().buffer().path();
                if let Some(path) = current_path {
                    self.lsp_manager.request_completion(path, position)?;
                }
            }
            Dispatch::DocumentDidChange { path, content } => {
                self.lsp_manager.document_did_change(path, content)?;
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
        self.add_and_focus_component(Rc::new(RefCell::new(prompt)));
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
        self.add_and_focus_component(Rc::new(RefCell::new(prompt)));
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

    fn open_file(&mut self, entry_path: PathBuf) -> anyhow::Result<()> {
        // TODO: check if the file is opened before
        // so that we won't notify the LSP twice
        let buffer = Rc::new(RefCell::new(Buffer::from_path(&entry_path)));
        self.buffers.push(buffer.clone());
        let entry_component = Rc::new(RefCell::new(SuggestiveEditor::from_buffer(buffer)));
        self.suggestive_editors.push(entry_component.clone());
        self.add_and_focus_component(entry_component);

        self.lsp_manager.open_file(entry_path.clone())?;

        Ok(())
    }

    fn handle_lsp_notification(&mut self, notification: LspNotification) -> anyhow::Result<()> {
        match notification {
            LspNotification::CompletionResponse(completion_response) => {
                let items = match completion_response {
                    lsp_types::CompletionResponse::Array(completion_items) => completion_items,
                    lsp_types::CompletionResponse::List(list) => list.items,
                };

                if let Some(editor) = self
                    .suggestive_editors
                    .iter()
                    .find(|editor| editor.borrow().id() == self.focused_component_id)
                {
                    log::info!("Setting items: {:?}", items.len());

                    editor.borrow_mut().set_completion(items);
                }
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
        }
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

#[derive(Clone)]
/// Dispatch are for child component to request action from the root node
pub enum Dispatch {
    CloseCurrentWindow { change_focused_to: ComponentId },
    SetSearch { search: String },
    OpenFile { path: PathBuf },
    RequestCompletion { position: Position },
    DocumentDidChange { path: PathBuf, content: String },
}

#[derive(Debug)]
pub enum ScreenMessage {
    LspNotification(LspNotification),
    Event(Event),
}
