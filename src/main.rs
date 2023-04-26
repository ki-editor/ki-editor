mod engine;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{EnableMouseCapture, MouseButton, MouseEventKind};
use crossterm::queue;
use crossterm::style::Print;
use crossterm::{cursor::SetCursorStyle, event::Event, terminal};
use log::LevelFilter;

use engine::{CharIndex, State};
use ropey::RopeSlice;
use std::io::{stdout, Write};
use std::path::Path;
use tree_sitter::{Parser, Point};

fn main() {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info).unwrap();
    let args = std::env::args().collect::<Vec<_>>();
    let filename = Path::new(args.get(1).unwrap());
    let content = std::fs::read_to_string(&filename).unwrap();
    let language = match filename.extension().unwrap().to_str().unwrap() {
        "js" => tree_sitter_javascript::language(),
        "rs" => tree_sitter_rust::language(),
        _ => panic!("Unsupported file extension"),
    };

    handle_event(&content, language)
}

use crossterm::{
    cursor::MoveTo,
    style::{Color, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};

use crossterm::{
    event::read,
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::engine::{CursorDirection, Mode};

fn handle_event(source_code: &str, language: tree_sitter::Language) {
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    let tree = parser.parse(source_code, None).unwrap();
    enable_raw_mode().unwrap();

    let mut state = State::new(source_code.into(), tree);
    let (columns, rows) = terminal::size().unwrap();
    let mut view = View {
        scroll_offset: 0,
        column_count: columns,
        row_count: rows,
        stdout: stdout(),
    };

    view.stdout.execute(EnableMouseCapture).unwrap();
    view.render(&state).unwrap();
    loop {
        let event = read().unwrap();
        match event {
            Event::Key(event) => state.handle_key_event(event),
            Event::Resize(columns, rows) => {
                view.set_columns(columns);
                view.set_row(rows)
            }
            Event::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::ScrollUp => {
                    view.scroll_offset = view.scroll_offset.saturating_sub(1)
                }
                MouseEventKind::ScrollDown => {
                    view.scroll_offset = view.scroll_offset.saturating_add(1)
                }
                MouseEventKind::Down(MouseButton::Left) => state
                    .set_cursor_position(mouse_event.row + view.scroll_offset, mouse_event.column),
                _ => continue,
            },
            _ => {
                log::info!("{:?}", event);

                // Don't render for unknown events
                continue;
            }
        }
        if state.quit {
            view.stdout.execute(Clear(ClearType::All)).unwrap();
            break;
        }
        view.render(&state).unwrap();
        view.stdout.flush().unwrap();
    }
    disable_raw_mode().unwrap();
}

struct View {
    /// Zero-based index.
    /// 2 means the first line to be rendered on the screen if the 3rd line of the text.
    scroll_offset: u16,

    row_count: u16,
    column_count: u16,
    stdout: std::io::Stdout,
}

impl View {
    fn set_columns(&mut self, columns: u16) {
        self.column_count = columns;
    }

    fn set_row(&mut self, rows: u16) {
        self.row_count = rows;
    }

    fn move_cursor(&mut self, point: Point) -> Result<(), anyhow::Error> {
        // Hide the cursor if the point is out of view
        if (point.row as u16).saturating_sub(self.scroll_offset) >= self.row_count {
            queue!(self.stdout, Hide)?;
        } else {
            queue!(self.stdout, Show)?;
            queue!(
                self.stdout,
                MoveTo(
                    point.column as u16,
                    (point.row as u16).saturating_sub(self.scroll_offset as u16)
                )
            )?;
        }
        Ok(())
    }

    fn get_grid(&self, state: &State) -> Vec<Vec<Cell>> {
        let mut grid: Vec<Vec<Cell>> = vec![];
        grid.resize_with(self.row_count.into(), || {
            let mut cells = vec![];
            cells.resize_with(self.column_count.into(), || Cell::default());
            cells
        });

        let lines = state
            .text
            .lines()
            .enumerate()
            .skip(self.scroll_offset.into())
            .take(self.row_count as usize - 1)
            .collect::<Vec<(_, RopeSlice)>>();

        let selection = &state.selection;
        let start_char_index = selection.start.0;
        let end_char_index = selection.end.0;
        let extended_selection = state.get_extended_selection();

        for (line_index, line) in lines {
            let line_start_char_index = CharIndex(state.text.line_to_char(line_index));
            for (local_char_index, c) in line.chars().take(self.column_count as usize).enumerate() {
                let char_index = line_start_char_index + local_char_index;

                let char_index = char_index.0;
                let background_color = if let Some(ref extended_selection) = extended_selection {
                    let x_start_point = extended_selection.start.0;
                    let x_end_point = extended_selection.end.0;
                    if start_char_index <= char_index
                        && char_index < end_char_index
                        && x_start_point <= char_index
                        && char_index < x_end_point
                    {
                        Color::Green
                    } else if x_start_point <= char_index && char_index < x_end_point {
                        Color::Cyan
                    } else if start_char_index <= char_index && char_index < end_char_index {
                        Color::Yellow
                    } else {
                        Color::Reset
                    }
                } else if start_char_index <= char_index && char_index < end_char_index {
                    Color::Yellow
                } else {
                    Color::Reset
                };
                grid[line_index - self.scroll_offset as usize][local_char_index] = Cell {
                    symbol: c.to_string(),
                    background_color,
                    foreground_color: Color::Black,
                };
            }
        }

        for (index, jump) in state.jumps().into_iter().enumerate() {
            let point = match state.cursor_direction {
                CursorDirection::Start => jump.selection.start,
                CursorDirection::End => jump.selection.end,
            }
            .to_point(&state.text);

            let column = point.column as u16;
            let row = point.row as u16 - self.scroll_offset as u16;

            // Background color: Odd index red, even index blue
            let background_color = if index % 2 == 0 {
                Color::Red
            } else {
                Color::Blue
            };

            // If column and row is within view
            if column < self.column_count && row < self.row_count {
                grid[row as usize][column as usize] = Cell {
                    symbol: jump.character.to_string(),
                    background_color,
                    foreground_color: Color::White,
                };
            }
        }
        grid
    }

    fn render(&mut self, state: &State) -> Result<(), anyhow::Error> {
        queue!(self.stdout, Clear(ClearType::All))?;
        let grid = self.get_grid(state);
        for (row, line) in grid.into_iter().enumerate() {
            for (column, cell) in line.into_iter().enumerate() {
                queue!(self.stdout, MoveTo(column as u16, row as u16))?;
                queue!(
                    self.stdout,
                    SetBackgroundColor(cell.background_color),
                    SetForegroundColor(cell.foreground_color),
                    Print(cell.symbol)
                )?;
            }
        }
        let point = state.get_cursor_point();
        self.move_cursor(point)?;

        match state.mode {
            Mode::Insert => {
                queue!(self.stdout, SetCursorStyle::BlinkingBar)?;
            }
            _ => {
                queue!(self.stdout, SetCursorStyle::SteadyBar)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Cell {
    pub symbol: String,
    pub foreground_color: Color,
    pub background_color: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: " ".to_string(),
            foreground_color: Color::Reset,
            background_color: Color::Reset,
        }
    }
}
