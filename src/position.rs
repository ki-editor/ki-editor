use crate::{buffer::Buffer, selection::CharIndex};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
pub(crate) struct Position {
    /// 0-based
    pub(crate) line: usize,
    /// 0-based
    pub(crate) column: usize,
}

impl Position {
    pub(crate) fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
    pub(crate) fn to_char_index(self, buffer: &Buffer) -> anyhow::Result<CharIndex> {
        buffer.position_to_char(self)
    }

    pub(crate) fn sub_column(&self, column: usize) -> Self {
        Self {
            line: self.line,
            column: self.column.saturating_sub(column),
        }
    }

    pub(crate) fn move_right(&self, by: u16) -> Position {
        Position {
            line: self.line,
            column: self.column + by as usize,
        }
    }

    pub(crate) fn move_up(&self, by: usize) -> Position {
        Position {
            line: self.line.saturating_sub(by),
            column: self.column,
        }
    }

    pub(crate) fn move_left(&self, by: usize) -> Position {
        Position {
            line: self.line,
            column: self.column.saturating_sub(by),
        }
    }

    pub(crate) fn set_line(self, line: usize) -> Position {
        Position { line, ..self }
    }

    pub(crate) fn move_down(&self, line: usize) -> Position {
        Position {
            line: self.line + line,
            column: self.column,
        }
    }

    pub(crate) fn translate(&self, Position { line, column }: Position) -> Position {
        Position {
            line: self.line + line,
            column: self.column + column,
        }
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => self.column.cmp(&other.column),
            ord => ord,
        }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<lsp_types::Position> for Position {
    fn from(value: lsp_types::Position) -> Self {
        Position {
            line: value.line as usize,
            column: value.character as usize,
        }
    }
}

impl From<Position> for lsp_types::Position {
    fn from(value: Position) -> Self {
        lsp_types::Position {
            line: value.line as u32,
            character: value.column as u32,
        }
    }
}

impl From<Position> for tree_sitter::Point {
    fn from(value: Position) -> Self {
        tree_sitter::Point {
            row: value.line,
            column: value.column,
        }
    }
}

impl From<tree_sitter::Point> for Position {
    fn from(value: tree_sitter::Point) -> Self {
        Position {
            line: value.row,
            column: value.column,
        }
    }
}
