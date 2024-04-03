use std::ops::Range;

use crate::{
    buffer::Buffer, components::editor::Direction, edit::is_overlapping, selection::CharIndex,
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
    // `edited_range` comes after
    if edited_range.start >= range.end {
        Some(range)
    }
    // `edited_range` comes before
    else if edited_range.end <= range.start {
        if change.is_positive() {
            let change = change as usize;
            Some(range.start + change..range.end + change)
        } else {
            let change = change.unsigned_abs();
            Some(range.start - change..range.end - change)
        }
    }
    // `edited_range` is equal or superset
    else if edited_range.start <= range.start && edited_range.end >= range.end {
        None
    }
    // `edited_range` is subset
    else if range.start <= edited_range.start && range.end >= edited_range.end {
        Some(range.start..(range.end as isize + change) as usize)
    }
    // `edited_range` intersects front
    else if range.contains(&edited_range.end) {
        Some(edited_range.end..range.end)
    }
    // `edited_range` intessects back
    else if range.contains(&edited_range.start) {
        Some(range.start..edited_range.start)
    } else {
        None
    }
}

#[cfg(test)]
mod test_apply_edit {
    use super::apply_edit;
    #[test]
    fn none_if_edited_range_is_equal() {
        assert_eq!(apply_edit(10..20, &(10..20), 0), None)
    }

    #[test]
    fn none_if_edited_range_is_superset() {
        assert_eq!(apply_edit(10..20, &(9..20), 0), None);
        assert_eq!(apply_edit(10..20, &(10..21), 0), None);
    }

    #[test]
    fn unaffected_if_edited_range_comes_after() {
        assert_eq!(apply_edit(10..20, &(20..30), 0), Some(10..20))
    }

    #[test]
    fn adjusted_by_offset_if_edited_range_comes_before() {
        assert_eq!(apply_edit(10..20, &(0..5), 3), Some(13..23));
        assert_eq!(apply_edit(10..20, &(0..5), -3), Some(7..17))
    }

    #[test]
    fn trim_front_if_edited_range_intersects_front() {
        assert_eq!(apply_edit(10..20, &(0..12), 3), Some(12..20));
    }

    #[test]
    fn trim_back_if_edited_range_intersects_back() {
        assert_eq!(apply_edit(10..20, &(18..22), 3), Some(10..18));
    }

    #[test]
    fn resize_if_edited_range_is_subset() {
        assert_eq!(apply_edit(10..20, &(11..12), 3), Some(10..23));
        assert_eq!(apply_edit(10..20, &(11..13), -1), Some(10..19));
    }
}
