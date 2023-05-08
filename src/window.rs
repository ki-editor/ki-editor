use std::io::Write;

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::EnableMouseCapture,
    queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use ropey::{Rope, RopeSlice};
use tree_sitter::Point;

use crate::{
    engine::{Buffer, CursorDirection, Mode},
    screen::Rectangle,
    selection::CharIndex,
};

pub struct Window {
    /// Zero-based index.
    /// 2 means the first line to be rendered on the screen if the 3rd line of the text.
    scroll_offset: u16,

    /// Used for diffing to reduce unnecessary re-painting.
    previous_grid: Option<Grid>,
    buffer_id: usize,
}

impl Window {
    fn move_cursor(
        &mut self,
        point: Point,
        rectangle: &Rectangle,
        stdout: &mut std::io::Stdout,
    ) -> Result<(), anyhow::Error> {
        // Hide the cursor if the point is out of view
        if !(0 as isize..rectangle.height as isize)
            .contains(&(point.row as isize - self.scroll_offset as isize))
        {
            queue!(stdout, Hide)?;
        } else {
            queue!(stdout, Show)?;
            queue!(
                stdout,
                MoveTo(
                    rectangle.origin.column as u16 + point.column as u16,
                    (rectangle.origin.row as u16 + (point.row as u16))
                        .saturating_sub(self.scroll_offset as u16)
                )
            )?;
        }
        Ok(())
    }

    fn get_grid(&self, height: usize, width: usize, buffer: &Buffer) -> Grid {
        let mut grid: Grid = Grid::new((height, width));

        let lines = buffer
            .text
            .lines()
            .enumerate()
            .skip(self.scroll_offset.into())
            .take(height - 1)
            .collect::<Vec<(_, RopeSlice)>>();

        let selection = &buffer.selection_set.primary;
        let secondary_selections = &buffer.selection_set.secondary;
        let extended_selection = buffer.get_extended_selection();

        for (line_index, line) in lines {
            let line_start_char_index = CharIndex(buffer.text.line_to_char(line_index));
            for (column_index, c) in line.chars().take(width).enumerate() {
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
                        secondary_selection.to_char_index(&buffer.cursor_direction) == char_index
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

        for (index, jump) in buffer.jumps().into_iter().enumerate() {
            let point = match buffer.cursor_direction {
                CursorDirection::Start => jump.selection.range.start,
                CursorDirection::End => jump.selection.range.end,
            }
            .to_point(&buffer.text);

            let column = point.column as u16;
            let row = (point.row as u16).saturating_sub(self.scroll_offset as u16);

            // Background color: Odd index red, even index blue
            let background_color = if index % 2 == 0 {
                Color::Red
            } else {
                Color::Blue
            };

            // If column and row is within view
            if column < width as u16 && row < height as u16 {
                grid.rows[row as usize][column as usize] = Cell {
                    symbol: jump.character.to_string(),
                    background_color,
                    foreground_color: Color::White,
                };
            }
        }

        grid
    }

    pub fn render(
        &mut self,
        buffer: &Buffer,
        rectangle: &Rectangle,
        stdout: &mut std::io::Stdout,
    ) -> Result<(), anyhow::Error> {
        queue!(stdout, Hide)?;
        let cells = {
            let grid = self.get_grid(rectangle.height, rectangle.width, buffer);

            let diff = if let Some(previous_grid) = self.previous_grid.take() {
                previous_grid.diff(&grid)
            } else {
                // queue!(stdout, Clear(ClearType::All)).unwrap();
                grid.to_position_cells()
            };

            self.previous_grid = Some(grid);

            diff
        };

        // TODO: remove this line
        let cells = self
            .get_grid(rectangle.height, rectangle.width, buffer)
            .to_position_cells();

        for cell in cells.into_iter() {
            queue!(
                stdout,
                MoveTo(
                    rectangle.origin.column as u16 + cell.position.column as u16,
                    rectangle.origin.row as u16 + cell.position.row as u16
                )
            )?;
            queue!(
                stdout,
                SetBackgroundColor(cell.cell.background_color),
                SetForegroundColor(cell.cell.foreground_color),
                Print(reveal(cell.cell.symbol))
            )?;
        }
        let point = buffer.get_cursor_point();
        self.move_cursor(point, rectangle, stdout)?;

        match buffer.mode {
            Mode::Insert => {
                queue!(stdout, SetCursorStyle::BlinkingBar)?;
            }
            _ => {
                queue!(stdout, SetCursorStyle::SteadyBar)?;
            }
        }

        Ok(())
    }

    pub fn new(buffer_id: usize) -> Self {
        Window {
            scroll_offset: 0,
            previous_grid: None,
            buffer_id,
        }
    }

    pub fn apply_scroll(&mut self, scroll_height: isize) {
        self.scroll_offset = if scroll_height.is_positive() {
            self.scroll_offset.saturating_add(scroll_height as u16)
        } else {
            self.scroll_offset
                .saturating_sub(scroll_height.abs() as u16)
        };
    }

    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    pub fn clear(&mut self, stdout: &mut std::io::Stdout) {
        stdout.execute(Clear(ClearType::All)).unwrap();
    }

    pub fn flush(&mut self, stdout: &mut std::io::Stdout) {
        stdout.flush().unwrap();
    }

    pub fn buffer_id(&self) -> usize {
        self.buffer_id
    }
}

#[derive(Clone, Debug)]
struct Grid {
    rows: Vec<Vec<Cell>>,
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

    fn new((height, width): (usize, usize)) -> Grid {
        let mut cells: Vec<Vec<Cell>> = vec![];
        cells.resize_with(height.into(), || {
            let mut cells = vec![];
            cells.resize_with(width.into(), || Cell::default());
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

#[cfg(test)]
mod test_grid {
    use tree_sitter::Point;

    use pretty_assertions::assert_eq;

    use crate::window::{Cell, Grid, PositionedCell};

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

/// Convert invisible character to visible character
fn reveal(s: String) -> String {
    match s.as_str() {
        "\n" => " ".to_string(),
        _ => s,
    }
}
