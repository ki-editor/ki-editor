use crate::{
    app::Dimension,
    position::Position,
    soft_wrap::{self},
    style::Style,
    themes::{Color, HighlightName, Theme},
};

use itertools::Itertools;
use my_proc_macros::hex;
#[cfg(test)]
use ropey::Rope;
use strum::IntoEnumIterator;
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Grid {
    pub(crate) rows: Vec<Vec<Cell>>,
    pub(crate) width: usize,
}

const DEFAULT_TAB_SIZE: usize = 4;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub(crate) struct Cell {
    pub(crate) symbol: char,
    pub(crate) foreground_color: Color,
    pub(crate) background_color: Color,
    pub(crate) line: Option<CellLine>,
    pub(crate) is_cursor: bool,
    /// Need for folded sections where the cursor is not in them
    pub(crate) is_protected_range_start: bool,
    /// For debugging purposes, so that we can trace this Cell is updated by which
    /// decoration, e.g. Diagnostic
    pub(crate) source: Option<StyleKey>,
    pub(crate) is_bold: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, PartialOrd, Ord)]
pub(crate) struct CellLine {
    pub(crate) color: Color,
    pub(crate) style: CellLineStyle,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy, PartialOrd, Ord)]
pub(crate) enum CellLineStyle {
    Undercurl,
    Underline,
}

impl Cell {
    #[cfg(test)]
    pub(crate) fn from_char(c: char) -> Self {
        Cell {
            symbol: c,
            ..Default::default()
        }
    }

    fn apply_update(&self, update: CellUpdate) -> Cell {
        Cell {
            symbol: update.symbol.unwrap_or(self.symbol),
            foreground_color: update
                .style
                .foreground_color
                .unwrap_or(self.foreground_color),
            background_color: update
                .style
                .background_color
                .unwrap_or(self.background_color),
            line: update.style.line.or(self.line),
            is_cursor: update.is_cursor || self.is_cursor,
            source: update.source.or_else(|| self.source.clone()),
            is_bold: update.style.is_bold || self.is_bold,
            is_protected_range_start: update.is_protected_range_start
                || self.is_protected_range_start,
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            symbol: ' ',
            foreground_color: hex!("#ffffff"),
            background_color: hex!("#ffffff"),
            line: None,
            is_cursor: false,
            source: None,
            is_bold: false,
            is_protected_range_start: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CellUpdate {
    pub(crate) position: Position,
    pub(crate) symbol: Option<char>,
    pub(crate) style: Style,
    pub(crate) is_cursor: bool,

    /// For debugging purposes
    pub(crate) source: Option<StyleKey>,
    pub(crate) is_protected_range_start: bool,
}

impl CellUpdate {
    pub(crate) fn new(position: Position) -> Self {
        CellUpdate {
            position,
            symbol: None,
            style: Style::default(),
            is_cursor: false,
            source: None,
            is_protected_range_start: false,
        }
    }

    pub(crate) fn set_is_cursor(self, is_cursor: bool) -> CellUpdate {
        CellUpdate { is_cursor, ..self }
    }

    pub(crate) fn set_position_line(self, line: usize) -> CellUpdate {
        CellUpdate {
            position: self.position.set_line(line),
            ..self
        }
    }

    #[cfg(test)]
    pub(crate) fn set_symbol(self, symbol: Option<char>) -> CellUpdate {
        CellUpdate { symbol, ..self }
    }

    pub(crate) fn set_is_protected_range_start(self, is_protected_range_start: bool) -> Self {
        CellUpdate {
            is_protected_range_start,
            ..self
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Hash)]
pub(crate) struct PositionedCell {
    pub(crate) cell: Cell,
    pub(crate) position: Position,
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

pub(crate) enum RenderContentLineNumber {
    NoLineNumber,
    LineNumber {
        /// 0-based
        start_line_index: usize,
        max_line_number: usize,
    },
}

impl Grid {
    pub(crate) fn new(dimension: Dimension) -> Grid {
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

    pub(crate) fn to_positioned_cells(&self) -> Vec<PositionedCell> {
        self.rows
            .iter()
            .enumerate()
            .flat_map(|(line, cells)| {
                cells
                    .iter()
                    .enumerate()
                    .map(|(column, cell)| PositionedCell {
                        cell: cell.clone(),
                        position: Position { line, column },
                    })
                    .collect_vec()
            })
            .collect_vec()
    }

    #[cfg(test)]
    pub(crate) fn from_text(dimension: Dimension, text: &str) -> Grid {
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
                        symbol: character,
                        ..Cell::default()
                    }
                })
        });

        grid
    }

    pub(crate) fn dimension(&self) -> Dimension {
        Dimension {
            height: self.rows.len() as u16,
            width: self.width as u16,
        }
    }

    pub(crate) fn apply_cell_update(mut self, update: CellUpdate) -> Grid {
        let Position { line, column } = update.position;
        if line < self.rows.len() && column < self.rows[line].len() {
            self.rows[line][column] = self.rows[line][column].apply_update(update);
        }
        self
    }

    pub(crate) fn apply_cell_updates(self, updates: Vec<CellUpdate>) -> Grid {
        updates
            .into_iter()
            .fold(self, |grid, update| grid.apply_cell_update(update))
    }

    pub(crate) fn merge_vertical(self, bottom: Grid) -> Grid {
        let mut top = self;
        top.rows.extend(bottom.rows);
        top
    }

    pub(crate) fn clamp_bottom(self, by: u16) -> Grid {
        let mut grid = self;
        let dimension = grid.dimension();
        let height = dimension.height.saturating_sub(by);

        if dimension.height > height {
            grid.rows.truncate(height as usize);
        }
        grid
    }

    pub(crate) fn get_cursor_position(&self) -> Option<Position> {
        self.to_positioned_cells().into_iter().find_map(|cell| {
            if cell.cell.is_cursor {
                Some(cell.position)
            } else {
                None
            }
        })
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
                    symbol: Some(character),
                    style: *style,
                    ..CellUpdate::default()
                }
            })
            .collect_vec()
    }

    /// This function handles a few things:
    /// - wrapping
    /// - Unicode width
    /// - line numbers
    ///
    /// Note:
    /// - `line_index_start` is 0-based.
    /// - If `max_line_number` is
    pub(crate) fn render_content(
        self,
        content: &str,
        line_number: RenderContentLineNumber,
        cell_updates: Vec<CellUpdate>,
        line_updates: Vec<LineUpdate>,
        theme: &Theme,
    ) -> Grid {
        let Dimension { height, width } = self.dimension();
        let (line_index_start, max_line_number_len, line_number_separator_width) = match line_number
        {
            RenderContentLineNumber::NoLineNumber => (0, 0, 0),
            RenderContentLineNumber::LineNumber {
                start_line_index: start_line_number,
                max_line_number,
            } => (
                start_line_number,
                max_line_number.max(1).to_string().len(),
                1,
            ),
        };
        let content_container_width = (width as usize)
            .saturating_sub(max_line_number_len)
            .saturating_sub(line_number_separator_width);

        let wrapped_lines = soft_wrap::soft_wrap(content, content_container_width);
        let content_cell_updates = {
            content
                .lines()
                .enumerate()
                .flat_map(|(line_index, line)| {
                    line.chars()
                        .enumerate()
                        .map(move |(column_index, character)| CellUpdate {
                            position: Position {
                                line: line_index,
                                column: column_index,
                            },
                            symbol: Some(character),
                            style: Style::default().foreground_color(theme.ui.text_foreground),
                            ..CellUpdate::default()
                        })
                })
                .map(|cell_update| CalibratableCellUpdate {
                    cell_update,
                    should_be_calibrated: true,
                })
        };
        let line_updates = line_updates
            .into_iter()
            .filter_map(|line_update| {
                let line = wrapped_lines
                    .calibrate(Position::new(line_update.line_index, 0))
                    .ok()?
                    .first()?
                    .line;
                Some((0..width).map(move |column_index| CalibratableCellUpdate {
                    should_be_calibrated: false,
                    cell_update: CellUpdate {
                        style: line_update.style,
                        position: Position {
                            line,
                            column: column_index as usize,
                        },
                        ..Default::default()
                    },
                }))
            })
            .flatten();
        let cell_updates = cell_updates
            .into_iter()
            .map(|cell_update| CalibratableCellUpdate {
                cell_update,
                should_be_calibrated: true,
            })
            .collect_vec();
        #[derive(Clone)]
        struct LineNumber {
            line_number: usize,
            wrapped: bool,
        }
        let line_numbers = wrapped_lines
            .lines()
            .iter()
            .flat_map(|line| {
                let line_number = line.line_number();
                line.lines()
                    .into_iter()
                    .enumerate()
                    .map(|(index, _)| LineNumber {
                        line_number: line_number + line_index_start,
                        wrapped: index > 0,
                    })
                    .collect_vec()
            })
            .collect::<Vec<_>>();
        let line_numbers = if line_numbers.is_empty() {
            [LineNumber {
                line_number: 0,
                wrapped: false,
            }]
            .to_vec()
        } else {
            line_numbers
        };
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
            match line_number {
                RenderContentLineNumber::NoLineNumber => Vec::new(),
                RenderContentLineNumber::LineNumber {
                    start_line_index: _,
                    max_line_number: _,
                } => line_numbers
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
                                    width = { max_line_number_len }
                                )
                            };
                            grid.get_row_cell_updates(
                                line_index,
                                Some(0),
                                Some(max_line_number_len),
                                &line_number_str,
                                &theme.ui.line_number,
                            )
                            .into_iter()
                            .chain(grid.get_row_cell_updates(
                                line_index,
                                Some(max_line_number_len),
                                Some(max_line_number_len + 1),
                                "│",
                                &theme.ui.border,
                            ))
                            .map(|cell_update| {
                                CalibratableCellUpdate {
                                    cell_update,
                                    should_be_calibrated: false,
                                }
                            })
                        },
                    )
                    .collect_vec(),
            }
        };
        let calibrated = content_cell_updates
            .into_iter()
            .chain(line_updates)
            .chain(cell_updates)
            .chain(line_numbers)
            .flat_map(|update| {
                if update.should_be_calibrated {
                    if let Ok(calibrated_position) =
                        wrapped_lines.calibrate(update.cell_update.position)
                    {
                        Box::new(calibrated_position.into_iter().enumerate().map(
                            move |(index, position)| CellUpdate {
                                position: position.move_right(
                                    (max_line_number_len + line_number_separator_width) as u16,
                                ),
                                symbol: if index == 0 {
                                    update.cell_update.symbol
                                } else {
                                    // Fill extra paddings with no-symbol cells
                                    None
                                },
                                ..update.cell_update.clone()
                            },
                        )) as Box<dyn Iterator<Item = CellUpdate>>
                    } else {
                        Box::new(std::iter::empty::<CellUpdate>())
                            as Box<dyn Iterator<Item = CellUpdate>>
                    }
                } else {
                    Box::new(std::iter::once(update.cell_update))
                        as Box<dyn Iterator<Item = CellUpdate>>
                }
            })
            .collect_vec();
        let protected_range_start_position = calibrated
            .iter()
            .find(|update| update.is_protected_range_start)
            .map(|update| update.position);
        // If the cursor is out of bound due to wrapped lines above it,
        // trim the lines from above until the cursor is inbound again
        let trimmed = if let Some(calibrated_cursor_position) = protected_range_start_position {
            let min_line = calibrated
                .iter()
                .map(|update| update.position.line)
                .min()
                .unwrap_or_default();
            let extra_height = calibrated_cursor_position
                .line
                .saturating_sub(min_line)
                .saturating_add(1)
                .saturating_sub(height as usize);

            let min_renderable_line = min_line + extra_height;
            calibrated
                .into_iter()
                .filter(|update| update.position.line >= min_renderable_line)
                .filter_map(|update| {
                    Some(CellUpdate {
                        position: update.position.move_up(extra_height)?,
                        ..update
                    })
                })
                .collect_vec()
        } else {
            calibrated
        };

        let result_grid = self
            .set_background_color(theme.ui.background_color)
            .apply_cell_updates(trimmed);
        debug_assert_eq!(result_grid.dimension().height, height);
        result_grid
    }

    fn set_background_color(mut self, background_color: Color) -> Self {
        for row in self.rows.iter_mut() {
            for cell in row {
                cell.background_color = background_color
            }
        }
        self
    }

    pub(crate) fn get_protected_range_start_position(&self) -> Option<Position> {
        self.to_positioned_cells().into_iter().find_map(|cell| {
            if cell.cell.is_protected_range_start {
                Some(cell.position)
            } else {
                None
            }
        })
    }

    pub(crate) fn height(&self) -> usize {
        self.rows.len()
    }
}

/// This is created so the payload of `StyleKey::Syntax`
/// can be usize instead of String.
///
/// The impact of this is huge because for a 3k Rust files
/// there are over 50,000 highlight spans,
/// and copying the style key as strings are very expensive.
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub(crate) struct IndexedHighlightGroup(usize);
impl IndexedHighlightGroup {
    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }

    #[cfg(test)]
    pub(crate) fn from_str(name: &str) -> Option<Self> {
        crate::themes::highlight_names()
            .iter()
            .position(|highlight_name| highlight_name == &name)
            .map(Self)
    }

    pub(crate) fn to_highlight_name(&self) -> Option<crate::themes::HighlightName> {
        HighlightName::iter().nth(self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub(crate) enum StyleKey {
    Syntax(IndexedHighlightGroup),
    UiPrimarySelection,

    UiPrimarySelectionAnchors,
    UiSecondarySelection,
    UiSecondarySelectionAnchors,
    DiagnosticsHint,
    DiagnosticsError,
    DiagnosticsWarning,
    DiagnosticsInformation,
    UiMark,
    UiPossibleSelection,

    DiagnosticsDefault,
    HunkOld,
    HunkOldEmphasized,
    HunkNew,
    HunkNewEmphasized,
    KeymapHint,
    KeymapArrow,
    KeymapKey,
    UiFuzzyMatchedChar,
    ParentLine,
    UiPrimarySelectionSecondaryCursor,
    UiSecondarySelectionPrimaryCursor,
    UiSecondarySelectionSecondaryCursor,
    UiSectionDivider,
    UiFocusedTab,
}

impl StyleKey {
    #[cfg(test)]
    pub(crate) fn display(&self) -> String {
        match self {
            StyleKey::Syntax(highlight_group) => {
                let str: &'static str = highlight_group.to_highlight_name().unwrap().into();
                str.to_string()
            }
            _ => format!("{self:?}"),
        }
    }
}

/// TODO: in the future, tab size should be configurable
pub(crate) fn get_string_width(str: &str) -> usize {
    str.chars().map(get_char_width).sum()
}

pub(crate) fn get_char_width(c: char) -> usize {
    match c {
        '\t' => DEFAULT_TAB_SIZE,
        _ => UnicodeWidthChar::width(c).unwrap_or(1),
    }
}

#[derive(Clone)]
pub(crate) struct LineUpdate {
    /// 0-based
    pub(crate) line_index: usize,
    pub(crate) style: Style,
}

#[cfg(test)]
mod test_grid {

    use my_proc_macros::hex;
    use pretty_assertions::assert_eq;

    use crate::{
        app::Dimension,
        grid::{CellUpdate, Grid, Style},
        position::Position,
    };

    use super::get_string_width;

    mod render_content {
        use crate::{
            grid::{Cell, LineUpdate, PositionedCell, RenderContentLineNumber},
            themes::Theme,
        };

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
            .render_content(
                "hello",
                RenderContentLineNumber::LineNumber {
                    max_line_number: 1,
                    start_line_index: 1,
                },
                Vec::new(),
                Vec::new(),
                &Theme::default(),
            )
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
                RenderContentLineNumber::LineNumber {
                    max_line_number: 10,
                    start_line_index: 10,
                },
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
                width: 8,
            })
            .render_content(
                "hello tim",
                RenderContentLineNumber::LineNumber {
                    max_line_number: 0,
                    start_line_index: 0,
                },
                Vec::new(),
                Vec::new(),
                &Theme::default(),
            )
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
                RenderContentLineNumber::LineNumber {
                    max_line_number: 1,
                    start_line_index: 1,
                },
                [CellUpdate {
                    symbol: Some(cursor),
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
                RenderContentLineNumber::LineNumber {
                    max_line_number: 1,
                    start_line_index: 1,
                },
                [CellUpdate {
                    symbol: Some(cursor),
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
            .render_content(
                "hello",
                RenderContentLineNumber::LineNumber {
                    max_line_number: 100,
                    start_line_index: 1,
                },
                [].to_vec(),
                Vec::new(),
                &Theme::default(),
            )
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
                "hello",
                RenderContentLineNumber::LineNumber {
                    max_line_number: 1,
                    start_line_index: 1,
                },
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
                (0..10)
                    .filter(|column| {
                        // Remove the column index 1, because it is the line number separator
                        // Which is not affected by the line update
                        column != &1
                    })
                    .collect_vec()
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
                    RenderContentLineNumber::LineNumber {
                        max_line_number: 0,
                        start_line_index: 0,
                    },
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

        #[test]
        /// No line number
        fn case_8() {
            let grid = Grid::new(Dimension {
                height: 1,
                width: 10,
            });
            let actual = grid
                .render_content(
                    "hello",
                    RenderContentLineNumber::NoLineNumber,
                    Vec::new(),
                    Vec::new(),
                    &Default::default(),
                )
                .to_string();
            assert_eq!("hello", actual)
        }

        #[test]
        /// Tab width
        fn case_9() {
            let grid = Grid::new(Dimension {
                height: 1,
                width: 8,
            });
            let actual = grid
                .render_content(
                    "\thel",
                    RenderContentLineNumber::NoLineNumber,
                    Vec::new(),
                    Vec::new(),
                    &Default::default(),
                )
                .to_positioned_cells()
                .into_iter()
                .map(|cell| cell.cell.symbol)
                .collect_vec();
            assert_eq!(['\t', ' ', ' ', ' ', 'h', 'e', 'l', ' '].to_vec(), actual)
        }

        #[test]
        /// Keep cursor in view if it has been pushed down by wrapped lines
        /// by trimming content from the top
        fn case_10() {
            let grid = Grid::new(Dimension {
                height: 2,
                width: 7,
            });
            let actual = grid
                .render_content(
                    "
1st line is long
x
"
                    .trim(),
                    RenderContentLineNumber::NoLineNumber,
                    [CellUpdate {
                        position: Position::new(1, 0), // on 'x'
                        is_cursor: true,
                        is_protected_range_start: true,
                        ..Default::default()
                    }]
                    .to_vec(),
                    Vec::new(),
                    &Default::default(),
                )
                .to_string();
            assert_eq!(
                actual,
                "
long
x
"
                .trim()
            )
        }

        #[test]
        /// Line update style should take precedence over content style
        fn case_11() {
            let grid = Grid::new(Dimension {
                height: 2,
                width: 7,
            });
            let color = hex!("#bababa");
            let actual = grid
                .render_content(
                    "hello",
                    RenderContentLineNumber::NoLineNumber,
                    Vec::new(),
                    [LineUpdate {
                        line_index: 0,
                        style: Style::default().foreground_color(color),
                    }]
                    .to_vec(),
                    &Default::default(),
                )
                .to_positioned_cells()
                .into_iter()
                .filter(|cell| cell.position == Position::new(0, 0))
                .collect_vec();
            let expected = PositionedCell {
                cell: Cell {
                    symbol: 'h',
                    foreground_color: color,
                    ..Default::default()
                },
                position: Position::default(),
            };
            assert_eq!(actual, [expected].to_vec())
        }
    }

    #[test]
    fn test_get_string_width() {
        assert_eq!(get_string_width("\t\t"), 8)
    }
}

#[cfg(test)]
mod test_cell {
    use super::*;
    #[test]
    fn apply_update() {
        let cell = Cell {
            symbol: 'a',
            foreground_color: hex!("#aaaaaa"),
            background_color: hex!("#bbbbbb"),
            line: Some(CellLine {
                color: hex!("#cccccc"),
                style: CellLineStyle::Undercurl,
            }),
            is_cursor: false,
            source: Some(StyleKey::HunkNew),
            is_bold: true,
            is_protected_range_start: false,
        };
        let cell = cell.apply_update(CellUpdate {
            position: Position::default(),
            symbol: Some('b'),
            style: Style::new()
                .foreground_color(hex!("#dddddd"))
                .background_color(hex!("#eeeeee"))
                .line(Some(CellLine {
                    color: hex!("#ffffff"),
                    style: CellLineStyle::Underline,
                })),
            is_cursor: true,
            source: Some(StyleKey::KeymapHint),
            is_protected_range_start: true,
        });
        assert_eq!(cell.symbol, 'b');
        assert_eq!(cell.foreground_color, hex!("#dddddd"));
        assert_eq!(cell.background_color, hex!("#eeeeee"));
        assert!(cell.is_cursor);
        assert_eq!(cell.source, Some(StyleKey::KeymapHint));
        assert_eq!(
            cell.line,
            Some(CellLine {
                color: hex!("#ffffff"),
                style: CellLineStyle::Underline,
            })
        )
    }
}
