use itertools::Itertools;

use crate::{
    app::Dimension,
    grid::{Grid, PositionedCell},
    rectangle::{Border, Rectangle},
    style::Style,
};

#[derive(Default, Clone)]
pub(crate) struct Screen {
    windows: Vec<Window>,
    borders: Vec<Border>,
    cursor: Option<crate::components::component::Cursor>,
    memoized_positioned_cells: Option<Vec<PositionedCell>>,
    border_style: Style,
}

impl Screen {
    pub(crate) fn new(
        windows: Vec<Window>,
        borders: Vec<Border>,
        cursor: Option<crate::components::component::Cursor>,
        border_style: Style,
    ) -> Screen {
        Screen {
            windows,
            borders,
            cursor,
            memoized_positioned_cells: None,
            border_style,
        }
    }
    /// This takes a `&mut self` instead of a `&self` because memoization.
    /// Memoization is necessary because there are other functions that depends on the result of this function,
    /// for example `Screen::dimension`.
    pub(crate) fn get_positioned_cells(&mut self) -> Vec<PositionedCell> {
        if let Some(positioned_cells) = self.memoized_positioned_cells.clone() {
            positioned_cells
        } else {
            self.memoized_positioned_cells = Some(
                self.windows
                    .iter()
                    .flat_map(Window::to_positioned_cells)
                    .chain(
                        self.borders
                            .iter()
                            .flat_map(|border| border.to_positioned_cells(self.border_style)),
                    )
                    // Cells should be sorted reversed by column, so that multi-width character
                    // will not be overridden by blank character in terminal rendering
                    .sorted_by_key(|cell| (cell.position.line, -(cell.position.column as isize)))
                    .collect(),
            );
            self.get_positioned_cells()
        }
    }

    pub(crate) fn cursor(&self) -> Option<crate::components::component::Cursor> {
        self.cursor.clone()
    }

    /// The `new_screen` need not be the same size as the old screen (`self`).
    pub(crate) fn diff(&mut self, old_screen: &mut Screen) -> Vec<PositionedCell> {
        // We use `IndexSet` instead of `HashSet` because the latter does not preserve ordering,
        // which can cause re-render to flicker like old TV (at least on Kitty term)

        let new = self.get_positioned_cells();
        let old: indexmap::IndexSet<PositionedCell> =
            old_screen.get_positioned_cells().into_iter().collect();
        new.into_iter()
            .filter(|cell| !old.contains(cell))
            .collect_vec()
    }

    #[cfg(test)]
    pub(crate) fn stringify(&mut self) -> String {
        self.get_positioned_cells()
            .into_iter()
            .group_by(|cell| cell.position.line)
            .into_iter()
            .map(|(_, cells)| {
                let cells = cells
                    .into_iter()
                    .sorted_by(|a, b| a.position.column.cmp(&b.position.column))
                    .map(|cell| {
                        if cell.cell.is_cursor {
                            "█".to_string()
                        } else {
                            cell.cell.symbol
                        }
                    })
                    .join("")
                    .trim_end()
                    .to_string();
                cells
            })
            .join("\n")
    }

    pub(crate) fn dimension(&mut self) -> Dimension {
        let cells = self.get_positioned_cells();
        let max_column = cells
            .iter()
            .max_by_key(|cell| cell.position.column)
            .map(|cell| cell.position.column)
            .unwrap_or_default();
        let max_line = cells
            .iter()
            .max_by_key(|cell| cell.position.line)
            .map(|cell| cell.position.line)
            .unwrap_or_default();
        Dimension {
            width: (max_column + 1) as u16,
            height: (max_line + 1) as u16,
        }
    }

    pub(crate) fn add_window(mut self, window: Window) -> Screen {
        self.windows.push(window);
        self
    }
}

#[derive(Clone)]
pub(crate) struct Window {
    grid: Grid,
    rectangle: Rectangle,
}

impl Window {
    pub(crate) fn to_positioned_cells(&self) -> Vec<PositionedCell> {
        self.grid
            .to_positioned_cells()
            .into_iter()
            .map(|cell| PositionedCell {
                position: cell.position.translate(self.rectangle.origin),
                ..cell
            })
            .collect()
    }

    pub(crate) fn new(grid: Grid, rectangle: Rectangle) -> Self {
        Self { grid, rectangle }
    }
}

#[cfg(test)]
mod test_screen {
    use crate::{
        app::Dimension,
        grid::{Cell, Grid, PositionedCell},
        position::Position,
        screen::{Screen, Window},
    };

    #[test]
    fn diff_same_size() {
        let dimension = Dimension {
            height: 2,
            width: 4,
        };
        let rectangle = crate::rectangle::Rectangle {
            origin: Position::new(0, 0),
            width: dimension.width,
            height: dimension.height,
        };
        let mut old = Screen::new(
            [Window::new(
                Grid::from_text(dimension, "a\nbc"),
                rectangle.clone(),
            )]
            .to_vec(),
            Vec::new(),
            None,
            Default::default(),
        );
        let mut new = Screen::new(
            [Window::new(Grid::from_text(dimension, "bc"), rectangle)].to_vec(),
            Vec::new(),
            None,
            Default::default(),
        );
        let actual = new.diff(&mut old);
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
