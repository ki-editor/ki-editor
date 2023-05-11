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
    engine::{Buffer, BufferConfig, Dispatch, HandleKeyEventResult, Mode},
    rectangle::Rectangle,
    window::{Grid, Window},
};

pub struct Screen {
    windows: AutoKeyMap<Window>,
    focused_window_index: usize,
    buffers: AutoKeyMap<Buffer>,
    state: State,
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
        Screen {
            windows: AutoKeyMap::new(),
            state: State {
                terminal_dimension: Dimension { height, width },
                search: None,
            },
            focused_window_index: 0,
            buffers: AutoKeyMap::new(),
            previous_grid: None,
        }
    }

    pub fn run(&mut self, entry_buffer: Buffer) -> Result<(), anyhow::Error> {
        crossterm::terminal::enable_raw_mode()?;

        let buffer_id = self.add_buffer(entry_buffer);

        let mut stdout = stdout();
        self.add_window(Window::new(buffer_id));

        stdout.execute(EnableMouseCapture)?;

        self.render(&mut stdout, buffer_id)?;
        loop {
            // Pass event to focused window
            let window = self.windows.get_mut(self.focused_window_index).unwrap();
            let buffer = self.buffers.get_mut(window.buffer_id()).unwrap();
            let event = crossterm::event::read()?;

            match event {
                Event::Key(event) => match event.code {
                    KeyCode::Char('%') => {
                        let buffer_id = window.buffer_id().clone();
                        self.focused_window_index = self.windows.insert(Window::new(buffer_id));
                    }
                    KeyCode::Char('f') if event.modifiers == KeyModifiers::CONTROL => {
                        let focused_window_index = self.focused_window_index.clone();
                        let override_fn =
                            Box::new(move |event: KeyEvent, buffer: &Buffer| match event.code {
                                KeyCode::Enter => HandleKeyEventResult::Consumed(vec![
                                    Dispatch::SetSearch {
                                        search: buffer.get_line().to_string(),
                                    },
                                    Dispatch::CloseCurrentWindow {
                                        change_focused_to: focused_window_index,
                                    },
                                ]),
                                _ => HandleKeyEventResult::Unconsumed(event),
                            });
                        let new_buffer = Buffer::from_config(
                            tree_sitter_md::language(),
                            "",
                            BufferConfig {
                                mode: Some(Mode::Insert),
                                normal_mode_override_fn: Some(override_fn.clone()),
                                insert_mode_override_fn: Some(override_fn),
                            },
                        );
                        let buffer_id = self.add_buffer(new_buffer);
                        self.focused_window_index = self.windows.insert(Window::new(buffer_id));
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
                        let dispatches = buffer.handle_key_event(&self.state, event);
                        self.handle_dispatches(dispatches)
                    }
                },
                Event::Resize(columns, rows) => {
                    self.state.terminal_dimension.height = rows;
                    self.state.terminal_dimension.width = columns;
                }
                Event::Mouse(mouse_event) => {
                    const SCROLL_HEIGHT: isize = 1;
                    match mouse_event.kind {
                        MouseEventKind::ScrollUp => {
                            window.apply_scroll(-SCROLL_HEIGHT);
                        }
                        MouseEventKind::ScrollDown => {
                            window.apply_scroll(SCROLL_HEIGHT);
                        }
                        MouseEventKind::Down(MouseButton::Left) => buffer.set_cursor_position(
                            mouse_event.row + window.scroll_offset(),
                            mouse_event.column,
                        ),
                        _ => continue,
                    }
                }
                _ => {
                    log::info!("Event = {:?}", event);

                    // Don't render for unknown events
                    continue;
                }
            }

            let current_buffer_id = self
                .windows
                .get(self.focused_window_index)
                .unwrap()
                .buffer_id();
            self.render(&mut stdout, current_buffer_id)?;
        }
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    fn add_buffer(&mut self, entry_buffer: Buffer) -> usize {
        self.buffers.insert(entry_buffer)
    }

    fn add_window(&mut self, buffer_id: Window) {
        self.windows.insert(buffer_id);
    }

    fn render(
        &mut self,
        stdout: &mut std::io::Stdout,
        current_buffer_id: usize,
    ) -> Result<(), anyhow::Error> {
        log::info!("Render");
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
                let buffer = self.buffers.get(window.buffer_id()).unwrap();
                let grid = window.get_grid(rectangle.dimension(), buffer);
                let cursor_point = if current_buffer_id == window.buffer_id() {
                    let cursor_position = buffer.get_cursor_point();
                    let scroll_offset = window.scroll_offset();

                    Some(Point::new(
                        cursor_position.row
                            + rectangle.origin.row.saturating_sub(scroll_offset as usize),
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
                // queue!(stdout, Clear(ClearType::All)).unwrap();
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

        log::info!("Cursor point = {:?}", cursor_point);
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
                self.windows.remove(self.focused_window_index);
                self.focused_window_index = change_focused_to;
            }
            Dispatch::SetSearch { search } => self.set_search(search),
        }
    }

    fn set_search(&mut self, search: String) {
        self.state.search = Some(search);
    }
}

#[derive(Debug, Clone, Copy)]
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
