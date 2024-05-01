use crate::{
    app::Dimension,
    position::Position,
    soft_wrap,
    style::Style,
    themes::{Color, Theme},
};

use itertools::Itertools;
use my_proc_macros::hex;
#[cfg(test)]
use ropey::Rope;
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub rows: Vec<Vec<Cell>>,
    pub width: usize,
}

const DEFAULT_TAB_SIZE: usize = 4;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Cell {
    pub symbol: String,
    pub foreground_color: Color,
    pub background_color: Color,
    pub line: Option<CellLine>,
    pub is_cursor: bool,
    /// For debugging purposes, so that we can trace this Cell is updated by which
    /// decoration, e.g. Diagnostic
    pub source: Option<StyleKey>,
    pub is_bold: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub struct CellLine {
    pub color: Color,
    pub style: CellLineStyle,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum CellLineStyle {
    Undercurl,
    Underline,
}

impl Cell {
    #[cfg(test)]
    pub fn from_char(c: char) -> Self {
        Cell {
            symbol: c.to_string(),
            ..Default::default()
        }
    }

    fn apply_update(&self, update: CellUpdate) -> Cell {
        Cell {
            symbol: update.symbol.clone().unwrap_or(self.symbol.clone()),
            foreground_color: update
                .style
                .foreground_color
                .unwrap_or(self.foreground_color),
            background_color: update
                .style
                .background_color
                .unwrap_or(self.background_color),
            line: self.line.or(update.style.line),
            is_cursor: update.is_cursor || self.is_cursor,
            source: update.source.or(self.source.clone()),
            is_bold: update.style.is_bold || self.is_bold,
        }
    }

    #[cfg(test)]
    fn set_background_color(self, background_color: Color) -> Cell {
        Cell {
            background_color,
            ..self
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: " ".to_string(),
            foreground_color: hex!("#ffffff"),
            background_color: hex!("#ffffff"),
            line: None,
            is_cursor: false,
            source: None,
            is_bold: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CellUpdate {
    pub position: Position,
    pub symbol: Option<String>,
    pub style: Style,
    pub is_cursor: bool,

    /// For debugging purposes
    pub source: Option<StyleKey>,
}

impl CellUpdate {
    pub fn new(position: Position) -> Self {
        CellUpdate {
            position,
            symbol: None,
            style: Style::default(),
            is_cursor: false,
            source: None,
        }
    }

    pub fn move_up(self, scroll_offset: usize) -> Option<CellUpdate> {
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

    pub fn set_is_cursor(self, is_cursor: bool) -> CellUpdate {
        CellUpdate { is_cursor, ..self }
    }

    pub fn set_position_line(self, line: usize) -> CellUpdate {
        CellUpdate {
            position: self.position.set_line(line),
            ..self
        }
    }

    pub(crate) fn move_right(self, by: u16) -> CellUpdate {
        CellUpdate {
            position: self.position.move_right(by),
            ..self
        }
    }

    #[cfg(test)]
    pub(crate) fn set_symbol(self, symbol: Option<String>) -> CellUpdate {
        CellUpdate { symbol, ..self }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Hash)]
pub struct PositionedCell {
    pub cell: Cell,
    pub position: Position,
}

impl PartialOrd for PositionedCell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PositionedCell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.position.cmp(&other.position)
    }
}
#[cfg(test)]
impl std::fmt::Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| cell.symbol.to_string())
                        .collect_vec()
                        .join("")
                        .trim()
                        .to_string()
                })
                .collect_vec()
                .join("\n")
        )
    }
}
impl Grid {
    pub fn new(dimension: Dimension) -> Grid {
        let mut cells: Vec<Vec<Cell>> = vec![];
        cells.resize_with(dimension.height.into(), || {
            let mut cells = vec![];
            cells.resize_with(dimension.width.into(), Cell::default);
            cells
        });
        Grid {
            rows: cells,
            width: dimension.width.into(),
        }
    }

    pub fn to_positioned_cells(&self) -> Vec<PositionedCell> {
        let mut cells = vec![];
        for (row_index, row) in self.rows.iter().enumerate() {
            let mut offset = 0;
            if row_index >= self.dimension().height as usize {
                return cells;
            }
            for (column_index, cell) in row.iter().enumerate() {
                let width = get_string_width(cell.symbol.as_str());
                let width = 1;
                for index in 0..width {
                    let adjusted_column = column_index + offset + index;
                    if adjusted_column >= self.width {
                        break;
                    }
                    cells.push(PositionedCell {
                        cell: Cell {
                            symbol: if index > 0 {
                                " ".to_string()
                            } else {
                                cell.symbol.clone()
                            },
                            ..cell.clone()
                        },
                        position: Position {
                            line: row_index,
                            column: adjusted_column,
                        },
                    });
                }
                offset += width.saturating_sub(1)
            }
        }

        cells
    }

    #[cfg(test)]
    pub fn from_text(dimension: Dimension, text: &str) -> Grid {
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

    pub fn dimension(&self) -> Dimension {
        Dimension {
            height: self.rows.len() as u16,
            width: self.width as u16,
        }
    }

    pub fn set_line(self, row: usize, title: &str, style: &Style) -> Grid {
        self.set_row(row, None, None, title, style)
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

    pub fn merge_vertical(self, bottom: Grid) -> Grid {
        let mut top = self;
        top.rows.extend(bottom.rows);
        top
    }

    pub fn clamp_bottom(self, by: u16) -> Grid {
        let mut grid = self;
        let dimension = grid.dimension();
        let height = dimension.height.saturating_sub(by);

        if dimension.height > height {
            grid.rows.truncate(height as usize);
        }
        grid
    }

    pub(crate) fn clamp_top(self, by: usize) -> Self {
        Self {
            rows: self.rows.into_iter().skip(by).collect_vec(),
            ..self
        }
    }

    pub fn get_cursor_position(&self) -> Option<Position> {
        self.to_positioned_cells().into_iter().find_map(|cell| {
            if cell.cell.is_cursor {
                Some(cell.position)
            } else {
                None
            }
        })
    }

    fn set_row(
        self,
        row_index: usize,
        column_start: Option<usize>,
        column_end: Option<usize>,
        content: &str,
        style: &Style,
    ) -> Self {
        let grid = self;
        // Trim or Pad end with spaces
        let cell_updates =
            grid.get_row_cell_updates(row_index, column_start, column_end, &content, style);
        grid.apply_cell_updates(cell_updates)
    }

    pub(crate) fn get_row_cell_updates(
        &self,
        row_index: usize,
        column_start: Option<usize>,
        column_end: Option<usize>,
        content: &str,
        style: &Style,
    ) -> Vec<CellUpdate> {
        let dimension = self.dimension();
        let grid = self;
        let column_range =
            column_start.unwrap_or(0)..column_end.unwrap_or(dimension.width as usize);
        // Trim or Pad end with spaces
        let content = format!("{:<width$}", content, width = column_range.len());
        let take = grid.dimension().width as usize;
        content
            .chars()
            .take(take)
            .enumerate()
            .map(|(char_index, character)| {
                let column_index = column_range.start + char_index;
                CellUpdate {
                    position: Position {
                        line: row_index,
                        column: column_index,
                    },
                    symbol: Some(character.to_string()),
                    style: *style,
                    ..CellUpdate::default()
                }
            })
            .collect_vec()
    }

    /// `line_index_start` is 0-based.
    pub(crate) fn render_content(
        self,
        content: &str,
        line_index_start: usize,
        max_line_number: usize,
        cell_updates: Vec<CellUpdate>,
        line_updates: Vec<LineUpdate>,
        theme: &Theme,
    ) -> Grid {
        let Dimension { height, width } = self.dimension();
        let max_line_number_len = max_line_number.max(1).to_string().len();

        let line_number_separator_width = 1;

        let content_container_width = ((width as usize)
            .saturating_sub(max_line_number_len)
            .saturating_sub(line_number_separator_width))
            as usize;

        let wrapped_lines = soft_wrap::soft_wrap(content, content_container_width);
        let content_cell_updates = {
            content.lines().enumerate().flat_map(|(line_index, line)| {
                line.chars()
                    .enumerate()
                    .map(move |(column_index, character)| CellUpdate {
                        position: Position {
                            line: line_index,
                            column: column_index,
                        },
                        symbol: Some(character.to_string()),
                        style: Style::default().foreground_color(theme.ui.text_foreground),
                        ..CellUpdate::default()
                    })
            })
        };
        let line_updates = line_updates.into_iter().flat_map(|line_update| {
            (0..width).map(move |column_index| CalibratableCellUpdate {
                should_be_calibrated: false,
                cell_update: CellUpdate {
                    style: line_update.style,
                    position: Position {
                        line: line_update.line_index,
                        column: column_index as usize,
                    },
                    ..Default::default()
                },
            })
        });
        let cell_updates = content_cell_updates
            .into_iter()
            .chain(cell_updates)
            .map(|cell_update| CalibratableCellUpdate {
                cell_update,
                should_be_calibrated: true,
            })
            .collect_vec();
        struct LineNumber {
            line_number: usize,
            wrapped: bool,
        }
        let lines = wrapped_lines
            .lines()
            .iter()
            .flat_map(|line| {
                let line_number = line.line_number();
                line.lines()
                    .into_iter()
                    .enumerate()
                    .map(|(index, _)| LineNumber {
                        line_number: line_number + (line_index_start as usize),
                        wrapped: index > 0,
                    })
                    .collect_vec()
            })
            .collect::<Vec<_>>();
        #[derive(Debug)]
        struct CalibratableCellUpdate {
            cell_update: CellUpdate,
            should_be_calibrated: bool,
        }
        let grid: Grid = Grid::new(Dimension {
            height: (height as usize).max(wrapped_lines.wrapped_lines_count()) as u16,
            width,
        });
        let line_numbers = {
            lines
                .into_iter()
                .enumerate()
                .flat_map(
                    |(
                        line_index,
                        LineNumber {
                            line_number,
                            wrapped,
                        },
                    )| {
                        let line_number_str = {
                            let line_number = if wrapped {
                                "↪".to_string()
                            } else {
                                (line_number + 1).to_string()
                            };
                            format!(
                                "{: >width$}",
                                line_number.to_string(),
                                width = max_line_number_len as usize
                            )
                        };
                        grid.get_row_cell_updates(
                            line_index,
                            Some(0),
                            Some(max_line_number_len as usize),
                            &line_number_str,
                            &theme.ui.line_number,
                        )
                        .into_iter()
                        .chain(grid.get_row_cell_updates(
                            line_index,
                            Some(max_line_number_len as usize),
                            Some((max_line_number_len + 1) as usize),
                            "│",
                            &theme.ui.line_number_separator,
                        ))
                        .map(|cell_update| CalibratableCellUpdate {
                            cell_update,
                            should_be_calibrated: false,
                        })
                    },
                )
                .collect_vec()
        };
        let calibrated = line_updates
            .into_iter()
            .chain(cell_updates)
            .chain(line_numbers)
            .filter_map(|update| {
                Some(if update.should_be_calibrated {
                    let calibrated_position = wrapped_lines
                        .calibrate(update.cell_update.position)
                        .ok()?
                        .move_right((max_line_number_len + line_number_separator_width) as u16);
                    CellUpdate {
                        position: calibrated_position,
                        ..update.cell_update
                    }
                } else {
                    update.cell_update
                })
            })
            .collect_vec();
        self.set_background_color(theme.ui.background_color)
            .apply_cell_updates(calibrated)
    }

    fn set_background_color(mut self, background_color: Color) -> Self {
        for row in self.rows.iter_mut() {
            for cell in row {
                cell.background_color = background_color
            }
        }
        self
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub enum StyleKey {
    Syntax(String),
    UiPrimarySelection,

    UiPrimarySelectionAnchors,
    UiSecondarySelection,
    UiSecondarySelectionAnchors,
    DiagnosticsHint,
    DiagnosticsError,
    DiagnosticsWarning,
    DiagnosticsInformation,
    UiBookmark,
    UiPossibleSelection,

    DiagnosticsDefault,
    HunkOld,
    HunkOldEmphasized,
    HunkNew,
    HunkNewEmphasized,
    KeymapHint,
    KeymapArrow,
    KeymapDescription,
    KeymapKey,
    UiFuzzyMatchedChar,
}

/// TODO: in the future, tab size should be configurable
pub fn get_string_width(str: &str) -> usize {
    str.chars()
        .map(|char| match char {
            '\t' => DEFAULT_TAB_SIZE,
            _ => UnicodeWidthChar::width(char).unwrap_or(1),
        })
        .sum()
}

#[derive(Clone)]
pub struct LineUpdate {
    /// 0-based
    pub line_index: usize,
    pub style: Style,
}

#[cfg(test)]
mod test_grid {

    use my_proc_macros::hex;
    use pretty_assertions::assert_eq;

    use crate::{
        app::Dimension,
        grid::{Cell, CellUpdate, Grid, PositionedCell, Style},
        position::Position,
    };

    use super::get_string_width;

    mod render_content {
        use crate::{grid::LineUpdate, themes::Theme};

        use super::*;
        use itertools::Itertools;
        use pretty_assertions::assert_eq;
        #[test]
        /// No wrap, no multi-width unicode
        fn case_1a() {
            let actual = Grid::new(Dimension {
                height: 1,
                width: 10,
            })
            .render_content("hello", 1, 1, Vec::new(), Vec::new(), &Theme::default())
            .to_string();
            assert_eq!(actual, "2│hello")
        }

        #[test]
        /// No wrap, no multi-width unicode, multiline
        fn case_1b() {
            let actual = Grid::new(Dimension {
                height: 2,
                width: 10,
            })
            .render_content(
                "hello\nworld",
                10,
                10,
                Vec::new(),
                Vec::new(),
                &Theme::default(),
            )
            .to_string();
            assert_eq!(
                actual,
                "
11│hello
12│world
"
                .trim()
            )
        }

        /// Wrapped, no multi-width unicode
        #[test]
        fn case_2() {
            let actual = Grid::new(Dimension {
                height: 2,
                width: 7,
            })
            .render_content("hello tim", 0, 0, Vec::new(), Vec::new(), &Theme::default())
            .to_string();
            assert_eq!(
                actual,
                "
1│hello
↪│ tim
"
                .trim()
            )
        }

        #[test]
        /// No wrap, with multi-width unicode
        fn case_3() {
            let crab = '🦀';
            let cursor = '█';
            assert_eq!(unicode_width::UnicodeWidthChar::width(crab), Some(2));
            let content = [crab, 'c', 'r', 'a', 'b'].into_iter().collect::<String>();
            let actual = Grid::new(Dimension {
                height: 1,
                width: 10,
            })
            .render_content(
                &content,
                1,
                1,
                [CellUpdate {
                    symbol: Some(cursor.to_string()),
                    position: Position::new(0, 3),
                    ..Default::default()
                }]
                .to_vec(),
                Vec::new(),
                &Theme::default(),
            )
            .to_string();
            // Expect a space is inserted between the crab emoji and 'c',
            // because the width of crab is 2
            assert_eq!(
                actual.chars().collect_vec(),
                ['2', '│', crab, ' ', 'c', 'r', cursor, 'b'].to_vec()
            )
        }

        #[test]
        /// Wrapped, with multi-width unicode
        fn case_4() {
            let crab = '🦀';
            let cursor = '█';
            assert_eq!(unicode_width::UnicodeWidthChar::width(crab), Some(2));
            let content = [
                crab, ' ', 'c', 'r', 'a', 'b', ' ', crab, ' ', 'c', 'r', 'a', 'b',
            ]
            .into_iter()
            .collect::<String>();
            let actual = Grid::new(Dimension {
                height: 4,
                width: 7,
            })
            .render_content(
                &content,
                1,
                1,
                [CellUpdate {
                    symbol: Some(cursor.to_string()),
                    position: Position::new(0, 8), // 3rd space
                    ..Default::default()
                }]
                .to_vec(),
                Vec::new(),
                &Theme::default(),
            )
            .to_string();
            assert_eq!(
                actual,
                "
2│🦀
↪│crab
↪│ 🦀 █
↪│crab
"
                .trim()
            )
        }

        #[test]
        /// Line number width should follow max_line_numbers_len
        fn case_5() {
            let actual = Grid::new(Dimension {
                height: 1,
                width: 10,
            })
            .render_content(&"hello", 1, 100, [].to_vec(), Vec::new(), &Theme::default())
            .to_string();
            // Expect there's two extra spaces before '2'
            // Because the number of digits of the last line is 3 ('1', '0', '0')
            assert_eq!(actual, "  2│hello".trim())
        }

        #[test]
        /// Line update
        fn case_6() {
            let color = hex!("#abcdef");
            let actual = Grid::new(Dimension {
                height: 1,
                width: 10,
            })
            .render_content(
                &"hello",
                1,
                1,
                [].to_vec(),
                [LineUpdate {
                    line_index: 0,
                    style: Style::default().background_color(color),
                }]
                .to_vec(),
                &Theme::default(),
            );
            assert_eq!(
                actual
                    .to_positioned_cells()
                    .into_iter()
                    .filter(|cell| cell.cell.background_color == color)
                    .map(|cell| cell.position.column)
                    .collect_vec(),
                (0..10).collect_vec()
            )
        }

        #[test]
        /// By default, background color of all cells should follow `theme.ui.background_color`.
        fn case_7() {
            let grid = Grid::new(Dimension {
                height: 1,
                width: 10,
            });
            let background_color = hex!("#bdfed2");
            let cells = grid
                .render_content(
                    "",
                    0,
                    0,
                    Vec::new(),
                    Vec::new(),
                    &Theme {
                        ui: crate::themes::UiStyles {
                            background_color,
                            ..Default::default()
                        },

                        ..Default::default()
                    },
                )
                .to_positioned_cells();
            assert_eq!(
                10,
                cells
                    .iter()
                    .filter(|cell| cell.cell.background_color == background_color)
                    .count()
            )
        }
    }

    #[test]
    fn set_row_should_pad_char_by_tab_width() {
        let dimension = Dimension {
            height: 1,
            width: 10,
        };
        let tab = '\t';
        let content = format!("{}x{}x", tab, tab);
        let tab_color = hex!("#abcdef");
        let x_color = hex!("#fafafa");
        let tab_style = Style::default().background_color(tab_color);
        let tab_cell_update = |column: usize| CellUpdate {
            position: Position { line: 0, column },
            symbol: None,
            style: tab_style,
            ..Default::default()
        };
        let x_cell_update = |column: usize| CellUpdate {
            position: Position { line: 0, column },
            symbol: None,
            style: Style::default().background_color(x_color),
            ..Default::default()
        };

        let grid = Grid::from_text(dimension, "")
            .set_row(0, None, None, &content, &Style::default())
            // Set the backgroud color of the first and second tab
            .apply_cell_update(tab_cell_update(0))
            .apply_cell_update(tab_cell_update(2))
            // Set the background color of the first and second 'x'
            .apply_cell_update(CellUpdate {
                is_cursor: true,
                ..x_cell_update(1)
            })
            .apply_cell_update(x_cell_update(3));
        let whitespace = |column: usize| PositionedCell {
            cell: Cell::from_char(' ').set_background_color(tab_color),
            position: Position { line: 0, column },
        };
        let tab = |column: usize| PositionedCell {
            cell: Cell::from_char('\t').set_background_color(tab_color),
            position: Position { line: 0, column },
        };
        let x = |column: usize, is_cursor: bool| PositionedCell {
            cell: Cell {
                is_cursor,
                ..Cell::from_char('x').set_background_color(x_color)
            },
            position: Position { line: 0, column },
        };
        let expected = [
            tab(0),
            // 3 whitespaces are added after tab, because the unicode width of tab is two
            whitespace(1),
            whitespace(2),
            whitespace(3),
            x(4, true),
            tab(5),
            whitespace(6),
            whitespace(7),
            whitespace(8),
            x(9, false),
        ]
        .to_vec();
        assert_eq!(grid.to_positioned_cells(), expected);
        let expected_cursor_position = Position { line: 0, column: 4 };
        assert_eq!(grid.get_cursor_position(), Some(expected_cursor_position));
    }

    #[test]
    fn set_row_should_pad_char_by_unicode_width() {
        use unicode_width::UnicodeWidthStr;

        let dimension = Dimension {
            height: 1,
            width: 3,
        };
        let microscope = '🔬';
        assert_eq!(UnicodeWidthStr::width(microscope.to_string().as_str()), 2); // Microscope
        let content = format!("{}x", microscope);
        let microscope_color = hex!("#abcdef");
        let x_color = hex!("#fafafa");
        let microscope_style = Style::default().background_color(microscope_color);

        let grid = Grid::from_text(dimension, "")
            .set_row(0, None, None, &content, &Style::default())
            .apply_cell_update(
                // Set the backgroud color of microscope
                CellUpdate {
                    position: Position { line: 0, column: 0 },
                    symbol: None,
                    style: microscope_style,
                    ..Default::default()
                },
            )
            .apply_cell_update(
                // Set the background color of 'x'
                CellUpdate {
                    position: Position { line: 0, column: 1 },
                    symbol: None,
                    style: Style::default().background_color(x_color),
                    ..Default::default()
                },
            );
        let expected = [
            PositionedCell {
                cell: Cell::from_char(microscope).set_background_color(microscope_color),
                position: Position { line: 0, column: 0 },
            },
            PositionedCell {
                // One whitespace is added after microscope, because the unicode width of microscope is two
                cell: Cell::from_char(' ').set_background_color(microscope_color),
                position: Position { line: 0, column: 1 },
            },
            PositionedCell {
                cell: Cell::from_char('x').set_background_color(x_color),
                position: Position { line: 0, column: 2 },
            },
        ]
        .to_vec();
        assert_eq!(grid.to_positioned_cells(), expected);
    }

    #[test]
    fn test_get_string_width() {
        assert_eq!(get_string_width("\t\t"), 8)
    }
}
