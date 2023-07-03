use crossterm::style::Color;

use crate::{
    position::Position,
    rectangle::{Border, BorderDirection, Rectangle},
    screen::Dimension,
};

#[cfg(test)]
use ropey::Rope;

#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub rows: Vec<Vec<Cell>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Cell {
    pub symbol: String,
    pub foreground_color: Color,
    pub background_color: Color,
}

impl Cell {
    #[cfg(test)]
    fn from_char(c: char) -> Self {
        Cell {
            symbol: c.to_string(),
            foreground_color: Color::White,
            background_color: Color::White,
        }
    }

    fn apply_update(&self, update: CellUpdate) -> Cell {
        Cell {
            symbol: update.symbol.unwrap_or(self.symbol.clone()),
            foreground_color: update.foreground_color.unwrap_or(self.foreground_color),
            background_color: update.background_color.unwrap_or(self.background_color),
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

pub struct CellUpdate {
    pub position: Position,
    pub symbol: Option<String>,
    pub background_color: Option<Color>,
    pub foreground_color: Option<Color>,
}

impl CellUpdate {
    pub fn new(position: Position) -> Self {
        CellUpdate {
            position,
            symbol: None,
            background_color: None,
            foreground_color: None,
        }
    }

    pub fn symbol(self, symbol: String) -> Self {
        CellUpdate {
            symbol: Some(symbol),
            ..self
        }
    }

    pub fn background_color(self, background_color: Color) -> Self {
        CellUpdate {
            background_color: Some(background_color),
            ..self
        }
    }

    pub fn foreground_color(self, foreground_color: Color) -> Self {
        CellUpdate {
            foreground_color: Some(foreground_color),
            ..self
        }
    }

    pub fn subtract_vertical_offset(self, scroll_offset: usize) -> Option<CellUpdate> {
        if scroll_offset > self.position.line {
            None
        } else {
            Some(CellUpdate {
                position: Position {
                    line: self.position.line - scroll_offset,
                    ..self.position
                },
                ..self
            })
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PositionedCell {
    pub cell: Cell,
    pub position: Position,
}

impl Grid {
    /// The `new_grid` need not be the same size as the old grid (`self`).
    pub fn diff(&self, new_grid: &Grid) -> Vec<PositionedCell> {
        let mut cells = vec![];
        for (row_index, new_row) in new_grid.rows.iter().enumerate() {
            for (column_index, new_cell) in new_row.iter().enumerate() {
                match self
                    .rows
                    .get(row_index)
                    .and_then(|old_row| old_row.get(column_index))
                {
                    Some(old_cell) if new_cell == old_cell => {
                        // Do nothing
                    }
                    // Otherwise
                    _ => cells.push(PositionedCell {
                        cell: new_cell.clone(),
                        position: Position {
                            line: row_index,
                            column: column_index,
                        },
                    }),
                }
            }
        }
        cells
    }

    pub fn new(dimension: Dimension) -> Grid {
        let mut cells: Vec<Vec<Cell>> = vec![];
        cells.resize_with(dimension.height.into(), || {
            let mut cells = vec![];
            cells.resize_with(dimension.width.into(), Cell::default);
            cells
        });
        Grid { rows: cells }
    }

    pub fn to_position_cells(&self) -> Vec<PositionedCell> {
        let mut cells = vec![];
        for (row_index, row) in self.rows.iter().enumerate() {
            for (column_index, cell) in row.iter().enumerate() {
                cells.push(PositionedCell {
                    cell: cell.clone(),
                    position: Position {
                        line: row_index,
                        column: column_index,
                    },
                })
            }
        }

        cells
    }

    #[cfg(test)]
    fn from_text(dimension: Dimension, text: &str) -> Grid {
        Grid::from_rope(dimension, &Rope::from_str(text))
    }

    #[cfg(test)]
    fn from_rope(dimension: Dimension, rope: &Rope) -> Grid {
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

    pub fn update(self, other: &Grid, rectangle: &Rectangle) -> Grid {
        let mut grid = self;
        for (row_index, rows) in other.rows.iter().enumerate() {
            for (column_index, cell) in rows.iter().enumerate() {
                grid.rows[row_index + rectangle.origin.row]
                    [column_index + rectangle.origin.column] = cell.clone();
            }
        }
        grid
    }

    pub fn set_border(mut self, border: &Border) -> Grid {
        let dimension = self.dimension();
        match border.direction {
            BorderDirection::Horizontal => {
                for i in 0..dimension.width.saturating_sub(border.start.column as u16) {
                    self.rows[border.start.row][border.start.column + i as usize] = Cell {
                        symbol: "─".to_string(),
                        foreground_color: Color::Black,
                        ..Cell::default()
                    };
                }
            }
            BorderDirection::Vertical => {
                for i in 0..dimension.height.saturating_sub(border.start.row as u16) {
                    self.rows[border.start.row + i as usize][border.start.column] = Cell {
                        symbol: "│".to_string(),
                        foreground_color: Color::Black,
                        ..Cell::default()
                    };
                }
            }
        }
        self
    }

    fn dimension(&self) -> Dimension {
        Dimension {
            height: self.rows.len() as u16,
            width: self.rows[0].len() as u16,
        }
    }

    pub fn set_line(self, row: usize, title: &str, style: Style) -> Grid {
        let mut grid = self;
        for (column_index, character) in title
            .chars()
            .take(grid.dimension().width as usize)
            .enumerate()
        {
            grid.rows[row][column_index] = Cell {
                symbol: character.to_string(),
                foreground_color: style.foreground_color,
                background_color: style.background_color,
            }
        }
        grid
    }

    pub fn apply_cell_update(mut self, update: CellUpdate) -> Grid {
        let Position { line, column } = update.position;
        if line < self.rows.len() && column < self.rows[line].len() {
            self.rows[line][column] = self.rows[line][column].apply_update(update);
        }
        self
    }

    pub fn apply_cell_updates(self, updates: Vec<CellUpdate>) -> Grid {
        updates
            .into_iter()
            .fold(self, |grid, update| grid.apply_cell_update(update))
    }
}

pub struct Style {
    pub foreground_color: Color,
    pub background_color: Color,
}

#[cfg(test)]
mod test_grid {

    use pretty_assertions::assert_eq;

    use crate::{
        grid::{Cell, Grid, PositionedCell},
        position::Position,
        screen::Dimension,
    };

    #[test]
    fn diff_same_size() {
        let dimension = Dimension {
            height: 2,
            width: 4,
        };
        let old = Grid::from_text(dimension, "a\nbc");
        let new = Grid::from_text(dimension, "bc");
        let actual = old.diff(&new);
        let expected = vec![
            PositionedCell {
                position: Position { line: 0, column: 0 },
                cell: Cell::from_char('b'),
            },
            PositionedCell {
                position: Position { line: 0, column: 1 },
                cell: Cell::from_char('c'),
            },
            PositionedCell {
                position: Position { line: 1, column: 0 },
                cell: Cell::from_char(' '),
            },
            PositionedCell {
                position: Position { line: 1, column: 1 },
                cell: Cell::from_char(' '),
            },
        ];
        assert_eq!(actual, expected);
    }
}
