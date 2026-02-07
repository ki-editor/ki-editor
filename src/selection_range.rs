// Change all usage of CharIndexRange, ByteRange, PositionRange etc to use this standardize representation

// So that we do not have to keep converting back and forth
use std::ops::Range;

use crate::{char_index_range::CharIndexRange, position::Position};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SelectionRange {
    Byte(Range<usize>),
    Position(Range<Position>),
}
impl SelectionRange {
    pub fn to_char_index_range(
        &self,
        buffer: &crate::buffer::Buffer,
    ) -> anyhow::Result<CharIndexRange> {
        match self {
            SelectionRange::Byte(byte_range) => buffer.byte_range_to_char_index_range(byte_range),
            SelectionRange::Position(position) => {
                buffer.position_range_to_char_index_range(position)
            }
        }
    }

    pub fn move_left(&self, count: usize) -> SelectionRange {
        match self {
            SelectionRange::Byte(_) => todo!(),
            SelectionRange::Position(range) => {
                Self::Position(range.start.move_left(count)..range.end.move_left(count))
            }
        }
    }
}

impl From<Range<Position>> for SelectionRange {
    fn from(v: Range<Position>) -> Self {
        Self::Position(v)
    }
}
