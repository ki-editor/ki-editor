use std::{cell::RefCell, io::stdout, rc::Rc};

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Print, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    ExecutableCommand,
};
use tree_sitter::Point;

use crate::{
    auto_key_map::AutoKeyMap,
    buffer::Buffer,
    engine::{Dispatch, Editor, EditorConfig, HandleKeyEventResult, Mode},
    grid::Grid,
    rectangle::{Border, Rectangle},
};

pub struct Screen {
    focused_editor_id: usize,

    editors: AutoKeyMap<Editor>,
    state: State,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    /// Used for diffing to reduce unnecessary re-painting.
    previous_grid: Option<Grid>,

    buffers: Vec<Rc<RefCell<Buffer>>>,
}

pub struct State {
    terminal_dimension: Dimension,
    search: Option<String>,
}
impl State {
    pub fn search(&self) -> &Option<String> {
        &self.search
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
                search: None,
            },
            focused_editor_id: 0,
            rectangles,
            borders,
            editors: AutoKeyMap::new(),
            previous_grid: None,
            buffers: Vec::new(),
        }
    }

    pub fn run(&mut self, entry_buffer: Buffer) -> Result<(), anyhow::Error> {
        crossterm::terminal::enable_raw_mode()?;

        let ref_cell = Rc::new(RefCell::new(entry_buffer));
        self.buffers.push(ref_cell.clone());
        let entry_editor = Editor::from_buffer(ref_cell);
        self.add_editor(entry_editor);

        let mut stdout = stdout();

        stdout.execute(EnableMouseCapture)?;

        self.render(&mut stdout)?;
        loop {
            // Pass event to focused window
            let editor = self.editors.get_mut(self.focused_editor_id).unwrap();
            let event = crossterm::event::read()?;

            match event {
                Event::Key(event) => match event.code {
                    KeyCode::Char('%') => {
                        let cloned = editor.clone();
                        self.focused_editor_id = self.add_editor(cloned);
                    }
                    KeyCode::Char('f') if event.modifiers == KeyModifiers::CONTROL => {
                        self.open_search_prompt()
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
                        let dispatches = editor.handle_key_event(&self.state, event);
                        self.handle_dispatches(dispatches)
                    }
                },
                Event::Resize(columns, rows) => {
                    self.resize(Dimension {
                        height: rows,
                        width: columns,
                    });
                }
                Event::Mouse(mouse_event) => {
                    editor.handle_mouse_event(mouse_event);
                }
                _ => {
                    log::info!("Event = {:?}", event);

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
        // Remove current editor
        self.editors.remove(self.focused_editor_id);
        if let Some((id, _)) = self.editors.entries().last() {
            self.focused_editor_id = *id;
            self.recalculate_layout();
            false
        } else {
            true
        }
    }

    fn add_editor(&mut self, entry_editor: Editor) -> usize {
        let editor_id = self.editors.insert(entry_editor);
        self.focused_editor_id = editor_id;
        self.recalculate_layout();
        editor_id
    }

    fn render(&mut self, stdout: &mut std::io::Stdout) -> Result<(), anyhow::Error> {
        // Generate layout
        let (rectangles, borders) =
            Rectangle::generate(self.editors.len(), self.state.terminal_dimension);

        let grid = Grid::new(self.state.terminal_dimension);

        // Render every window
        let (grid, cursor_point) = self
            .editors
            .entries()
            .zip(rectangles.into_iter())
            .map(|((editor_id, editor), rectangle)| {
                let grid = editor.get_grid();
                let cursor_point = if editor_id == &self.focused_editor_id {
                    let cursor_position = editor.get_cursor_point();
                    let scroll_offset = editor.scroll_offset();

                    // If cursor position is in view
                    if cursor_position.row < scroll_offset as usize
                        || cursor_position.row
                            >= (scroll_offset + rectangle.dimension().height) as usize
                    {
                        return (grid, rectangle, None);
                    }

                    Some(Point::new(
                        (cursor_position.row + rectangle.origin.row)
                            .saturating_sub(scroll_offset as usize),
                        cursor_position.column + rectangle.origin.column,
                    ))
                } else {
                    None
                };

                (grid, rectangle, cursor_point)
            })
            .fold(
                (grid, None),
                |(grid, current_cursor_point), (window_grid, rectangle, cursor_point)| {
                    (
                        grid.update(&window_grid, rectangle),
                        current_cursor_point.or_else(|| cursor_point),
                    )
                },
            );

        // Render every border
        let grid = borders
            .into_iter()
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

        // let cells = grid.to_position_cells();

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

    fn handle_dispatches(&mut self, dispatches: Vec<Dispatch>) {
        dispatches
            .into_iter()
            .for_each(|dispatch| self.handle_dispatch(dispatch))
    }

    fn handle_dispatch(&mut self, dispatch: Dispatch) {
        match dispatch {
            Dispatch::CloseCurrentWindow { change_focused_to } => {
                self.editors.remove(self.focused_editor_id);
                self.focused_editor_id = change_focused_to;
                self.recalculate_layout();
            }
            Dispatch::SetSearch { search } => self.set_search(search),
        }
    }

    fn set_search(&mut self, search: String) {
        self.state.search = Some(search);
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
            Rectangle::generate(self.editors.len(), self.state.terminal_dimension);
        self.rectangles = rectangles;
        self.borders = borders;

        self.editors
            .values_mut()
            .zip(self.rectangles.iter())
            .for_each(|(editor, rectangle)| editor.set_dimension(rectangle.dimension()));
    }

    fn open_search_prompt(&mut self) {
        let focused_editor_id = self.focused_editor_id.clone();
        let override_fn = Box::new(move |event: KeyEvent, editor: &Editor| match event.code {
            KeyCode::Enter => HandleKeyEventResult::Consumed(vec![
                Dispatch::SetSearch {
                    search: editor.get_line(),
                },
                Dispatch::CloseCurrentWindow {
                    change_focused_to: focused_editor_id,
                },
            ]),
            _ => HandleKeyEventResult::Unconsumed(event),
        });
        let new_editor = Editor::from_config(
            tree_sitter_md::language(),
            "",
            EditorConfig {
                mode: Some(Mode::Insert),
                normal_mode_override_fn: Some(override_fn.clone()),
                insert_mode_override_fn: Some(override_fn),
            },
        );
        let editor_id = self.add_editor(new_editor);
        self.focused_editor_id = editor_id
    }

    fn change_view(&mut self) {
        if let Some(id) = self
            .editors
            .keys()
            .find(|editor_id| editor_id > &&self.focused_editor_id)
            .map_or_else(|| self.editors.keys().min(), |id| Some(id))
        {
            self.focused_editor_id = id.clone();
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
