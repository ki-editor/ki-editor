mod edit;
mod engine;
mod selection;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{EnableMouseCapture, MouseButton, MouseEventKind};
use crossterm::queue;
use crossterm::style::Print;
use crossterm::{cursor::SetCursorStyle, event::Event, terminal};
use log::LevelFilter;

use engine::State;
use ropey::{Rope, RopeSlice};
use selection::CharIndex;
use std::io::{stdout, Write};
use std::path::Path;
use tree_sitter::{Parser, Point};

fn main() {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info).unwrap();
    let args = std::env::args().collect::<Vec<_>>();
    let filename = Path::new(args.get(1).unwrap());
    let content = std::fs::read_to_string(&filename).unwrap();
    let language = match filename.extension().unwrap().to_str().unwrap() {
        "js" | "ts" | "tsx" | "jsx" => tree_sitter_javascript::language(),
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
        previous_grid: None,
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
            Event::Mouse(mouse_event) => {
                const SCROLL_HEIGHT: u16 = 1;
                match mouse_event.kind {
                    MouseEventKind::ScrollUp => {
                        view.scroll_offset = view.scroll_offset.saturating_sub(SCROLL_HEIGHT)
                    }
                    MouseEventKind::ScrollDown => {
                        view.scroll_offset = view.scroll_offset.saturating_add(SCROLL_HEIGHT)
                    }
                    MouseEventKind::Down(MouseButton::Left) => state.set_cursor_position(
                        mouse_event.row + view.scroll_offset,
                        mouse_event.column,
                    ),
                    _ => continue,
                }
            }
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

#[derive(Clone, Debug)]
struct Grid {
    rows: Vec<Vec<Cell>>,
}

#[derive(Debug, PartialEq, Eq)]
struct PositionedCell {
    cell: Cell,
    position: Point,
}
impl Grid {
    /// Returns (rows, columns)
    fn dimensions(&self) -> (usize, usize) {
        (self.rows.len(), self.rows[0].len())
    }

    /// The `new_grid` need not be the same size as the old grid (`self`).
    fn diff(&self, new_grid: &Grid) -> Vec<PositionedCell> {
        let mut cells = vec![];
        for (row_index, new_row) in new_grid.rows.iter().enumerate() {
            for (column_index, new_cell) in new_row.iter().enumerate() {
                match self
                    .rows
                    .get(row_index)
                    .map(|old_row| old_row.get(column_index))
                    .flatten()
                {
                    Some(old_cell) if new_cell == old_cell => {
                        // Do nothing
                    }
                    // Otherwise
                    _ => cells.push(PositionedCell {
                        cell: new_cell.clone(),
                        position: Point::new(row_index as usize, column_index as usize),
                    }),
                }
            }
        }
        cells
    }

    fn new((row_count, column_count): (usize, usize)) -> Grid {
        let mut cells: Vec<Vec<Cell>> = vec![];
        cells.resize_with(row_count.into(), || {
            let mut cells = vec![];
            cells.resize_with(column_count.into(), || Cell::default());
            cells
        });
        Grid { rows: cells }
    }

    fn to_position_cells(&self) -> Vec<PositionedCell> {
        let mut cells = vec![];
        for (row_index, row) in self.rows.iter().enumerate() {
            for (column_index, cell) in row.iter().enumerate() {
                cells.push(PositionedCell {
                    cell: cell.clone(),
                    position: Point::new(row_index as usize, column_index as usize),
                })
            }
        }

        cells
    }

    fn from_text(dimension: (usize, usize), text: &str) -> Grid {
        Grid::from_rope(dimension, &Rope::from_str(text))
    }

    fn from_rope(dimension: (usize, usize), rope: &Rope) -> Grid {
        let mut grid = Grid::new(dimension);

        rope.lines().enumerate().for_each(|(row_index, line)| {
            line.chars()
                .enumerate()
                .for_each(|(column_index, character)| {
                    grid.rows[row_index][column_index] = Cell {
                        symbol: character.to_string(),
                        ..Cell::default()
                    }
                })
        });

        grid
    }
}

struct View {
    /// Zero-based index.
    /// 2 means the first line to be rendered on the screen if the 3rd line of the text.
    scroll_offset: u16,

    row_count: u16,
    column_count: u16,
    stdout: std::io::Stdout,

    previous_grid: Option<Grid>,
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

    fn get_grid(&self, state: &State) -> Grid {
        let mut grid: Grid = Grid::new((self.row_count as usize, self.column_count as usize));

        let lines = state
            .text
            .lines()
            .enumerate()
            .skip(self.scroll_offset.into())
            .take(self.row_count as usize - 1)
            .collect::<Vec<(_, RopeSlice)>>();

        let selection = &state.selection_set.primary;
        let secondary_selections = &state.selection_set.secondary;
        let extended_selection = state.get_extended_selection();

        for (line_index, line) in lines {
            let line_start_char_index = CharIndex(state.text.line_to_char(line_index));
            for (column_index, c) in line.chars().take(self.column_count as usize).enumerate() {
                let char_index = line_start_char_index + column_index;

                let (foreground_color, background_color) =
                    if let Some(ref extended_selection) = extended_selection {
                        if selection.range.contains(&char_index)
                            && extended_selection.range.contains(&char_index)
                        {
                            (Color::Black, Color::Green)
                        } else if extended_selection.range.contains(&char_index) {
                            (Color::Black, Color::Cyan)
                        } else if selection.range.contains(&char_index) {
                            (Color::Black, Color::Yellow)
                        } else {
                            (Color::Black, Color::White)
                        }
                    } else if selection.range.contains(&char_index) {
                        (Color::Black, Color::Yellow)
                    } else if secondary_selections.iter().any(|secondary_selection| {
                        secondary_selection.to_char_index(&state.cursor_direction) == char_index
                    }) {
                        (Color::White, Color::Black)
                    } else if secondary_selections
                        .iter()
                        .any(|secondary_selection| secondary_selection.range.contains(&char_index))
                    {
                        (Color::Black, Color::DarkYellow)
                    } else {
                        (Color::Black, Color::White)
                    };
                grid.rows[line_index - self.scroll_offset as usize][column_index] = Cell {
                    symbol: c.to_string(),
                    background_color,
                    foreground_color,
                };
            }
        }

        for (index, jump) in state.jumps().into_iter().enumerate() {
            let point = match state.cursor_direction {
                CursorDirection::Start => jump.selection.range.start,
                CursorDirection::End => jump.selection.range.end,
            }
            .to_point(&state.text);

            let column = point.column as u16;
            let row = (point.row as u16).saturating_sub(self.scroll_offset as u16);

            // Background color: Odd index red, even index blue
            let background_color = if index % 2 == 0 {
                Color::Red
            } else {
                Color::Blue
            };

            // If column and row is within view
            if column < self.column_count && row < self.row_count {
                grid.rows[row as usize][column as usize] = Cell {
                    symbol: jump.character.to_string(),
                    background_color,
                    foreground_color: Color::White,
                };
            }
        }

        grid
    }

    fn render(&mut self, state: &State) -> Result<(), anyhow::Error> {
        queue!(self.stdout, Hide)?;
        let cells = {
            let grid = self.get_grid(state);

            let diff = if let Some(previous_grid) = self.previous_grid.take() {
                previous_grid.diff(&grid)
            } else {
                queue!(self.stdout, Clear(ClearType::All)).unwrap();
                grid.to_position_cells()
            };

            self.previous_grid = Some(grid);

            diff
        };

        for cell in cells.into_iter() {
            queue!(
                self.stdout,
                MoveTo(cell.position.column as u16, cell.position.row as u16)
            )?;
            queue!(
                self.stdout,
                SetBackgroundColor(cell.cell.background_color),
                SetForegroundColor(cell.cell.foreground_color),
                Print(reveal(cell.cell.symbol))
            )?;
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

/// Convert invisible character to visible character
fn reveal(s: String) -> String {
    match s.as_str() {
        "\n" => " ".to_string(),
        _ => s,
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Cell {
    pub symbol: String,
    pub foreground_color: Color,
    pub background_color: Color,
}

impl Cell {
    fn from_char(c: char) -> Self {
        Cell {
            symbol: c.to_string(),
            foreground_color: Color::White,
            background_color: Color::White,
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: " ".to_string(),
            foreground_color: Color::White,
            background_color: Color::White,
        }
    }
}

#[cfg(test)]
mod test_grid {
    use tree_sitter::Point;

    use crate::{Cell, Grid, PositionedCell};
    use pretty_assertions::assert_eq;

    #[test]
    fn diff_same_size() {
        let old = Grid::from_text((2, 4), "a\nbc");
        let new = Grid::from_text((2, 4), "bc");
        let actual = old.diff(&new);
        let expected = vec![
            PositionedCell {
                position: Point { row: 0, column: 0 },
                cell: Cell::from_char('b'),
            },
            PositionedCell {
                position: Point { row: 0, column: 1 },
                cell: Cell::from_char('c'),
            },
            PositionedCell {
                position: Point { row: 1, column: 0 },
                cell: Cell::from_char(' '),
            },
            PositionedCell {
                position: Point { row: 1, column: 1 },
                cell: Cell::from_char(' '),
            },
        ];
        assert_eq!(actual, expected);
    }
}
