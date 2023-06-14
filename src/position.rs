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
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.line < other.line {
            Some(std::cmp::Ordering::Less)
        } else if self.line > other.line {
            Some(std::cmp::Ordering::Greater)
        } else {
            self.column.partial_cmp(&other.column)
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

impl Into<lsp_types::Position> for Position {
    fn into(self) -> lsp_types::Position {
        lsp_types::Position {
            line: self.line as u32,
            character: self.column as u32,
        }
    }
}

impl Into<tree_sitter::Point> for Position {
    fn into(self) -> tree_sitter::Point {
        tree_sitter::Point {
            row: self.line as usize,
            column: self.column as usize,
        }
    }
}

impl From<tree_sitter::Point> for Position {
    fn from(value: tree_sitter::Point) -> Self {
        Position {
            line: value.row as usize,
            column: value.column as usize,
        }
    }
}
