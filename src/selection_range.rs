// Change all usage of CharIndexRange, ByteRange, PositionRange etc to use this standardize representation

// So that we do not have to keep converting back and forth
use std::ops::Range;

use crate::{char_index_range::CharIndexRange, position::Position, selection::CharIndex};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SelectionRange {
    Byte(Range<usize>),
    CharIndex(Range<CharIndex>),
    Position(Range<Position>),
    /// 0-based index
    Line(usize),
}
impl SelectionRange {
    pub(crate) fn to_char_index_range(
        &self,
        buffer: &std::cell::Ref<'_, crate::buffer::Buffer>,
    ) -> anyhow::Result<CharIndexRange> {
        match self {
            SelectionRange::Byte(byte_range) => buffer.byte_range_to_char_index_range(byte_range),
            SelectionRange::CharIndex(range) => Ok(range.clone().into()),
            SelectionRange::Position(position) => {
                buffer.position_range_to_char_index_range(position)
            }
            SelectionRange::Line(line) => buffer.line_to_char_range(*line),
        }
    }

    pub(crate) fn move_left(&self, count: usize) -> SelectionRange {
        match self {
            SelectionRange::Byte(_) => todo!(),
            SelectionRange::CharIndex(_) => todo!(),
            SelectionRange::Position(range) => {
                Self::Position(range.start.move_left(count)..range.end.move_left(count))
            }
            SelectionRange::Line(_) => todo!(),
        }
    }
}

impl From<Range<Position>> for SelectionRange {
    fn from(v: Range<Position>) -> Self {
        Self::Position(v)
    }
}
