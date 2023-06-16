use crate::buffer::Buffer;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Ord)]
pub struct Position {
    /// 0-based
    pub line: usize,
    /// 0-based
    pub column: usize,
}
impl Position {
    pub fn to_char_index(self, buffer: &Buffer) -> crate::selection::CharIndex {
        buffer.position_to_char(self)
    }

    pub fn sub_column(&self, column: usize) -> Self {
        Self {
            line: self.line,
            column: self.column.saturating_sub(column),
        }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => Some(self.column.cmp(&other.column)),
            ord => Some(ord),
        }
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
