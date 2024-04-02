use std::ops::Range;

use crate::{
    buffer::Buffer,
    components::editor::Direction,
    edit::{is_overlapping, ApplyOffset},
    selection::CharIndex,
};

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

pub trait ToByteRange {
    fn to_byte_range(&self, buffer: &Buffer) -> anyhow::Result<Range<usize>>;
}

pub trait ToCharIndexRange {
    fn to_char_index_range(&self, buffer: &Buffer) -> anyhow::Result<CharIndexRange>;
}

impl ToCharIndexRange for Range<usize> {
    fn to_char_index_range(&self, buffer: &Buffer) -> anyhow::Result<CharIndexRange> {
        Ok((buffer.byte_to_char(self.start)?..buffer.byte_to_char(self.end)?).into())
    }
}

impl ToByteRange for CharIndexRange {
    fn to_byte_range(&self, buffer: &Buffer) -> anyhow::Result<Range<usize>> {
        Ok(buffer.char_to_byte(self.start)?..buffer.char_to_byte(self.end)?)
    }
}

impl CharIndexRange {
    pub fn iter(&self) -> CharIndexRangeIter {
        CharIndexRangeIter {
            range: *self,
            current: self.start,
        }
    }

    pub fn len(&self) -> usize {
        self.end.0.saturating_sub(self.start.0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn shift_left(&self, len: usize) -> CharIndexRange {
        CharIndexRange {
            start: self.start - len,
            end: self.end - len,
        }
    }

    pub fn cursor_position(&self, cursor_direction: &Direction) -> CharIndex {
        match cursor_direction {
            Direction::Start => self.start,
            Direction::End => self.end,
        }
    }

    fn overlaps(&self, other: &CharIndexRange) -> bool {
        is_overlapping(&self.to_range(), &other.to_range())
    }

    fn to_range(self) -> Range<CharIndex> {
        self.start..self.end
    }

    pub(crate) fn end(&self) -> CharIndex {
        self.end
    }

    pub(crate) fn start(&self) -> CharIndex {
        self.start
    }

    pub(crate) fn apply_edit(
        &self,
        range: &CharIndexRange,
        chars_offset: isize,
    ) -> Option<CharIndexRange> {
        let range = apply_edit(
            self.start.0..self.end.0,
            &(range.start.0..range.end.0),
            chars_offset,
        )?;
        Some((CharIndex(range.start)..(CharIndex(range.end))).into())
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

/// `change` = new length - old length
pub fn apply_edit(
    range: Range<usize>,
    edited_range: &Range<usize>,
    change: isize,
) -> Option<Range<usize>> {
    if edited_range.start >= range.end {
        Some(range)
    } else if is_overlapping(edited_range, &range) {
        None
    } else {
        if change.is_positive() {
            Some(range.start + (change as usize)..range.end + (change as usize))
        } else {
            Some(range.start - change.unsigned_abs()..range.end - change.unsigned_abs())
        }
    }
}
