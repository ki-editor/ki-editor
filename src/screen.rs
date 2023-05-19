use std::{
    cell::RefCell,
    io::stdout,
    path::{Path, PathBuf},
    rc::Rc,
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
use tree_sitter::Point;

use crate::{
    auto_key_map::AutoKeyMap,
    buffer::Buffer,
    components::{
        component::{Component, ComponentId},
        dropdown::{Dropdown, DropdownConfig},
        editor::{Direction, Editor},
        prompt::{Prompt, PromptConfig},
    },
    grid::{Grid, Style},
    rectangle::{Border, Rectangle},
};

pub struct Screen {
    focused_component_id: ComponentId,

    components: AutoKeyMap<ComponentId, Rc<RefCell<dyn Component>>>,
    state: State,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    /// Used for diffing to reduce unnecessary re-painting.
    previous_grid: Option<Grid>,

    buffers: Vec<Rc<RefCell<Buffer>>>,
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
    pub fn new() -> Screen {
        let (width, height) = terminal::size().unwrap();
        let dimension = Dimension { height, width };
        let (rectangles, borders) = Rectangle::generate(1, dimension);
        Screen {
            state: State {
                terminal_dimension: dimension,
                previous_searches: vec![],
            },
            focused_component_id: ComponentId(0),
            rectangles,
            borders,
            components: AutoKeyMap::new(),
            previous_grid: None,
            buffers: Vec::new(),
        }
    }

    pub fn run(&mut self, entry_path: PathBuf) -> Result<(), anyhow::Error> {
        crossterm::terminal::enable_raw_mode()?;

        self.open_file(entry_path);

        let mut stdout = stdout();

        stdout.execute(EnableMouseCapture)?;

        self.render(&mut stdout)?;
        loop {
            // Pass event to focused window
            let component = self.components.get_mut(self.focused_component_id).unwrap();
            let event = crossterm::event::read()?;

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
                            break;
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

                    // Don't render for unknown events
                    continue;
                }
            }

            self.render(&mut stdout)?;
        }
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    // Return true if there's no more windows
    fn quit(&mut self) -> bool {
        // Remove current component
        self.components.remove(self.focused_component_id);
        if let Some((id, _)) = self.components.entries().last() {
            self.focused_component_id = *id;
            self.recalculate_layout();
            false
        } else {
            true
        }
    }

    fn add_component(&mut self, entry_component: Rc<RefCell<dyn Component>>) -> ComponentId {
        let component_id = self.components.insert(entry_component);
        self.focused_component_id = component_id;
        self.recalculate_layout();
        component_id
    }

    fn render(&mut self, stdout: &mut std::io::Stdout) -> Result<(), anyhow::Error> {
        // Generate layout
        let grid = Grid::new(self.state.terminal_dimension);

        // Render every window
        let (grid, cursor_point) = self
            .components
            .entries()
            .map(|(component_id, component)| {
                let component = component.borrow();

                let rectangle = component.rectangle();
                let component_grid = component.get_grid();
                let cursor_point = if component_id == &self.focused_component_id {
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
            Ok(dispatches) => self.handle_dispatches(dispatches),
            Err(error) => {
                todo!("Show the error to the user")
            }
        }
    }

    fn handle_dispatches(&mut self, dispatches: Vec<Dispatch>) {
        dispatches
            .into_iter()
            .for_each(|dispatch| self.handle_dispatch(dispatch))
    }

    fn handle_dispatch(&mut self, dispatch: Dispatch) {
        match dispatch {
            Dispatch::CloseCurrentWindow { change_focused_to } => {
                self.close_current_window(change_focused_to)
            }
            Dispatch::SetSearch { search } => self.set_search(search),
            Dispatch::OpenFile { path } => self.open_file(path),
        }
    }

    fn current_component(&self) -> &Rc<RefCell<dyn Component>> {
        self.components.get(self.focused_component_id).unwrap()
    }

    fn close_current_window(&mut self, change_focused_to: ComponentId) {
        let current_component = self.current_component();
        let slave_ids = current_component.borrow().slave_ids();
        self.components.remove(self.focused_component_id);
        slave_ids.into_iter().for_each(|slave_id| {
            self.components.remove(slave_id);
        });

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
            Rectangle::generate(self.components.len(), self.state.terminal_dimension);
        self.rectangles = rectangles;
        self.borders = borders;

        self.components
            .values_mut()
            .zip(self.rectangles.iter())
            .for_each(|(component, rectangle)| {
                // Leave 1 row on top for rendering the title
                let (_, rectangle) = rectangle.split_vertically_at(1);
                component.borrow_mut().set_rectangle(rectangle.clone())
            });
    }

    fn open_search_prompt(&mut self) {
        let dropdown = Rc::new(RefCell::new(Dropdown::new(DropdownConfig {
            title: "Suggestions".to_string(),
        })));
        let owner_id = self.focused_component_id;
        let current_component = self.current_component().clone();
        let dropdown_id = self.add_component(dropdown.clone());
        let prompt = Prompt::new(PromptConfig {
            title: "Search".to_string(),
            owner_id,
            dropdown_id,
            history: self.state.previous_searches.clone(),
            dropdown,
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
        let component_id = self.add_component(Rc::new(RefCell::new(prompt)));
        self.focused_component_id = component_id;
    }

    fn open_file_picker(&mut self) {
        let dropdown = Rc::new(RefCell::new(Dropdown::new(DropdownConfig {
            title: "Matching files".to_string(),
        })));
        let owner_id = self.focused_component_id;
        let current_component = self.current_component().clone();
        let dropdown_id = self.add_component(dropdown.clone());
        let prompt = Prompt::new(PromptConfig {
            title: "Open File".to_string(),
            owner_id,
            dropdown_id,
            history: self.state.previous_searches.clone(),
            dropdown,
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
        let component_id = self.add_component(Rc::new(RefCell::new(prompt)));
        self.focused_component_id = component_id;
    }

    fn change_view(&mut self) {
        if let Some(id) = self
            .components
            .keys()
            .find(|component_id| component_id > &&self.focused_component_id)
            .map_or_else(|| self.components.keys().min(), |id| Some(id))
        {
            self.focused_component_id = id.clone();
        }
    }

    fn open_file(&mut self, entry_path: PathBuf) {
        let ref_cell = Rc::new(RefCell::new(Buffer::from_path(&entry_path)));
        self.buffers.push(ref_cell.clone());
        let entry_component = Rc::new(RefCell::new(Editor::from_buffer(ref_cell)));
        self.add_component(entry_component);
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
pub enum Dispatch {
    CloseCurrentWindow { change_focused_to: ComponentId },
    SetSearch { search: String },
    OpenFile { path: PathBuf },
}
