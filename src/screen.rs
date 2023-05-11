use std::io::stdout;

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{
        EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind,
    },
    execute, queue,
    style::{Print, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    ExecutableCommand,
};
use tree_sitter::Point;

use crate::{
    auto_key_map::AutoKeyMap,
    engine::{Dispatch, Editor, EditorConfig, HandleKeyEventResult, Mode},
    rectangle::{Border, Rectangle},
    window::{Grid, Window},
};

pub struct Screen {
    windows: AutoKeyMap<Window>,
    focused_window_index: usize,

    // TODO: buffers are actually windows, and windows is actually useless.
    // We don't have structure to represent the actual buffer yet
    editors: AutoKeyMap<Editor>,
    state: State,
    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,
    /// Used for diffing to reduce unnecessary re-painting.
    previous_grid: Option<Grid>,
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
            windows: AutoKeyMap::new(),
            state: State {
                terminal_dimension: dimension,
                search: None,
            },
            focused_window_index: 0,
            rectangles,
            borders,
            editors: AutoKeyMap::new(),
            previous_grid: None,
        }
    }

    pub fn run(&mut self, entry_editor: Editor) -> Result<(), anyhow::Error> {
        crossterm::terminal::enable_raw_mode()?;

        let editor_id = self.add_editor(entry_editor);

        let mut stdout = stdout();
        self.add_window(Window::new(editor_id));

        stdout.execute(EnableMouseCapture)?;

        self.render(&mut stdout, editor_id)?;
        loop {
            // Pass event to focused window
            let window = self.windows.get_mut(self.focused_window_index).unwrap();
            let editor = self.editors.get_mut(window.editor_id()).unwrap();
            let event = crossterm::event::read()?;

            match event {
                Event::Key(event) => match event.code {
                    KeyCode::Char('%') => {
                        let editor_id = window.editor_id().clone();
                        self.focused_window_index = self.windows.insert(Window::new(editor_id));
                    }
                    KeyCode::Char('f') if event.modifiers == KeyModifiers::CONTROL => {
                        self.open_search_prompt()
                    }
                    KeyCode::Char('q') if event.modifiers == KeyModifiers::CONTROL => {
                        // Remove current window
                        self.windows.remove(self.focused_window_index);
                        self.focused_window_index = self.focused_window_index.saturating_sub(1);

                        // TODO: remove this break
                        break;

                        continue;
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

            let current_editor_id = self
                .windows
                .get(self.focused_window_index)
                .unwrap()
                .editor_id();
            self.render(&mut stdout, current_editor_id)?;
        }
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    fn add_editor(&mut self, entry_editor: Editor) -> usize {
        let editor_id = self.editors.insert(entry_editor);
        self.recalculate_layout();
        editor_id
    }

    fn add_window(&mut self, editor_id: Window) {
        self.windows.insert(editor_id);
    }

    fn render(
        &mut self,
        stdout: &mut std::io::Stdout,
        current_editor_id: usize,
    ) -> Result<(), anyhow::Error> {
        // queue!(stdout, Clear(ClearType::All)).unwrap();
        // Generate layout
        let (rectangles, borders) =
            Rectangle::generate(self.windows.len(), self.state.terminal_dimension);

        let grid = Grid::new(self.state.terminal_dimension);

        // Render every window
        let (grid, cursor_point) = self
            .windows
            .values()
            .zip(rectangles.into_iter())
            .map(|(window, rectangle)| {
                let editor = self.editors.get(window.editor_id()).unwrap();
                let grid = window.get_grid(rectangle.dimension(), editor);
                let cursor_point = if current_editor_id == window.editor_id() {
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

        // queue!(stdout, MoveTo(0, 0))?;

        // self.move_cursor(point, rectangle, stdout)?;

        // match buffer.mode {
        //     Mode::Insert => {
        //         queue!(stdout, SetCursorStyle::BlinkingBar)?;
        //     }
        //     _ => {
        //         queue!(stdout, SetCursorStyle::SteadyBar)?;
        //     }
        // }

        Ok(())
    }

    // fn move_cursor(
    //     &mut self,
    //     point: Point,
    //     rectangle: &Rectangle,
    //     stdout: &mut std::io::Stdout,
    // ) -> Result<(), anyhow::Error> {
    //     // Hide the cursor if the point is out of view
    //     if !(0 as isize..rectangle.height as isize)
    //         .contains(&(point.row as isize - self.scroll_offset as isize))
    //     {
    //         queue!(stdout, Hide)?;
    //     } else {
    //         queue!(stdout, Show)?;
    //         queue!(
    //             stdout,
    //             MoveTo(
    //                 rectangle.origin.column as u16 + point.column as u16,
    //                 (rectangle.origin.row as u16 + (point.row as u16))
    //                     .saturating_sub(self.scroll_offset as u16)
    //             )
    //         )?;
    //     }
    //     Ok(())
    // }

    fn handle_dispatches(&mut self, dispatches: Vec<Dispatch>) {
        dispatches
            .into_iter()
            .for_each(|dispatch| self.handle_dispatch(dispatch))
    }

    fn handle_dispatch(&mut self, dispatch: Dispatch) {
        match dispatch {
            Dispatch::CloseCurrentWindow { change_focused_to } => {
                let current_window = self.windows.get(self.focused_window_index).unwrap();
                self.editors.remove(current_window.editor_id());
                self.windows.remove(self.focused_window_index);
                self.focused_window_index = change_focused_to;
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
        let focused_window_index = self.focused_window_index.clone();
        let override_fn = Box::new(move |event: KeyEvent, editor: &Editor| match event.code {
            KeyCode::Enter => HandleKeyEventResult::Consumed(vec![
                Dispatch::SetSearch {
                    search: editor.get_line().to_string(),
                },
                Dispatch::CloseCurrentWindow {
                    change_focused_to: focused_window_index,
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
        self.focused_window_index = self.windows.insert(Window::new(editor_id));
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
