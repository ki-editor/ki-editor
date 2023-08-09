use std::ops::Range;

use crate::{edit::ApplyOffset, selection::CharIndex};

#[derive(PartialEq, Clone, Debug, Eq, Hash, Default)]
pub struct CharIndexRange {
    pub start: CharIndex,
    pub end: CharIndex,
}

impl From<CharIndexRange> for Range<CharIndex> {
    fn from(val: CharIndexRange) -> Self {
        val.start..val.end
    }
}

impl CharIndexRange {
    pub fn apply_edit(self, edit: &crate::edit::Edit) -> CharIndexRange {
        if edit.start >= self.end {
            self
        } else {
            self.apply_offset(edit.offset())
        }
    }

    pub fn iter(&self) -> CharIndexRangeIter {
        CharIndexRangeIter {
            range: self.clone(),
            current: self.start,
        }
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
