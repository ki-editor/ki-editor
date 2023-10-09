// Change all usage of CharIndexRange, ByteRange, PositionRange etc to use this standardize representation

// So that we do not have to keep converting back and forth
use std::ops::Range;

use crate::{position::Position, selection::CharIndex};
enum SelectionRange {
    Byte(Range<usize>),
    CharIndex(Range<CharIndex>),
    Position(Range<Position>),
}

impl From<Range<Position>> for SelectionRange {
    fn from(v: Range<Position>) -> Self {
        Self::Position(v)
    }
}
