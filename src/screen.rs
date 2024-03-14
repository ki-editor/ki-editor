use itertools::Itertools;

use crate::{
    app::Dimension,
    grid::{Cell, Grid, PositionedCell},
    rectangle::{Border, Rectangle},
};

#[derive(Default, Clone)]
pub struct Screen {
    windows: Vec<Window>,
    borders: Vec<Border>,
    cursor: Option<crate::components::component::Cursor>,
}

impl Screen {
    pub fn new(
        windows: Vec<Window>,
        borders: Vec<Border>,
        cursor: Option<crate::components::component::Cursor>,
    ) -> Screen {
        Screen {
            windows,
            borders,
            cursor,
        }
    }
    pub fn to_positioned_cells(&self) -> Vec<PositionedCell> {
        self.windows
            .iter()
            .flat_map(Window::to_positioned_cells)
            .chain(
                self.borders
                    .iter()
                    .flat_map(|border| border.to_positioned_cells()),
            )
            .collect()
    }

    pub(crate) fn cursor(&self) -> Option<crate::components::component::Cursor> {
        self.cursor.clone()
    }

    /// The `new_screen` need not be the same size as the old screen (`self`).
    pub fn diff(&self, old_screen: &Screen) -> Vec<PositionedCell> {
        // We use `IndexSet` instead of `HashSet` because the latter does not preserve ordering,
        // which can cause re-render to flicker like old TV (at least on Kitty term)

        let new: indexmap::IndexSet<PositionedCell> =
            self.to_positioned_cells().into_iter().collect();
        let old: indexmap::IndexSet<PositionedCell> =
            old_screen.to_positioned_cells().into_iter().collect();
        new.difference(&old)
            .map(|cell| cell.to_owned())
            .collect_vec()
    }

    #[cfg(test)]
    pub(crate) fn to_string(&self) -> String {
        self.to_positioned_cells()
            .into_iter()
            .group_by(|cell| cell.position.line)
            .into_iter()
            .map(|(_, cells)| {
                let cells = cells
                    .into_iter()
                    .sorted_by(|a, b| a.position.column.cmp(&b.position.column))
                    .map(|cell| cell.cell.symbol)
                    .join("")
                    .trim_end()
                    .to_string();
                cells
            })
            .join("\n")
    }

    pub(crate) fn dimension(&self) -> Dimension {
        let cells = self.to_positioned_cells();
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
}

#[derive(Clone)]
pub struct Window {
    grid: Grid,
    rectangle: Rectangle,
}

impl Window {
    pub fn to_positioned_cells(&self) -> Vec<PositionedCell> {
        self.grid
            .to_positioned_cells()
            .into_iter()
            .map(|cell| PositionedCell {
                position: cell.position.translate(self.rectangle.origin),
                cell: Cell {
                    symbol: if cell.cell.is_cursor {
                        "█".to_string()
                    } else {
                        cell.cell.symbol
                    },
                    ..cell.cell
                },
                ..cell
            })
            // // Why is this necessary? Because some character takes up multiple terminal cells, such as tab
            // .filter(|cell| self.rectangle.in_bound(cell.position))
            .collect()
    }

    pub(crate) fn new(grid: Grid, rectangle: Rectangle) -> Self {
        Self { grid, rectangle }
    }
}
