use std::ops::Range;

use crate::{edit::ApplyOffset, selection::CharIndex};

#[derive(PartialEq, Clone, Debug, Eq, Hash, Default, Copy)]
pub struct CharIndexRange {
    pub start: CharIndex,
    pub end: CharIndex,
}

impl std::ops::Sub<usize> for CharIndexRange {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            start: self.start - rhs,
            end: self.end - rhs,
        }
    }
}

impl From<CharIndexRange> for Range<CharIndex> {
    fn from(val: CharIndexRange) -> Self {
        val.start..val.end
    }
}

impl CharIndexRange {
    pub fn apply_edit(self, edit: &crate::edit::Edit) -> CharIndexRange {
        if edit.range.start >= self.end {
            self
        } else {
            self.apply_offset(edit.offset())
        }
    }

    pub fn iter(&self) -> CharIndexRangeIter {
        CharIndexRangeIter {
            range: *self,
            current: self.start,
        }
    }

    pub fn len(&self) -> usize {
        self.end.0.saturating_sub(self.start.0)
    }
}

pub struct CharIndexRangeIter {
    range: CharIndexRange,
    current: CharIndex,
}

impl Iterator for CharIndexRangeIter {
    type Item = CharIndex;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.range.end {
            let result = self.current;
            self.current = self.current + 1;
            Some(result)
        } else {
            None
        }
    }
}

impl From<Range<CharIndex>> for CharIndexRange {
    fn from(value: Range<CharIndex>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}
