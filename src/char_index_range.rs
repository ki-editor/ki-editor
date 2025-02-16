use std::ops::Range;

use crate::{edit::Edit, selection::CharIndex};

#[derive(PartialEq, Clone, Debug, Eq, Hash, Default, Copy, PartialOrd, Ord)]
pub(crate) struct CharIndexRange {
    pub(crate) start: CharIndex,
    pub(crate) end: CharIndex,
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
    pub(crate) fn iter(&self) -> CharIndexRangeIter {
        CharIndexRangeIter {
            range: *self,
            current: self.start,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.end.0.saturating_sub(self.start.0)
    }

    pub(crate) fn shift_left(&self, len: usize) -> CharIndexRange {
        CharIndexRange {
            start: self.start - len,
            end: self.end - len,
        }
    }

    pub(crate) fn shift_right(&self, len: usize) -> CharIndexRange {
        CharIndexRange {
            start: self.start + len,
            end: self.end + len,
        }
    }

    pub(crate) fn end(&self) -> CharIndex {
        self.end
    }

    pub(crate) fn start(&self) -> CharIndex {
        self.start
    }

    pub(crate) fn apply_edit(&self, edit: &Edit) -> Option<CharIndexRange> {
        let range = apply_edit(
            self.start.0..self.end.0,
            &(edit.range.start.0..edit.range.end.0),
            edit.chars_offset(),
        )?;
        Some((CharIndex(range.start)..(CharIndex(range.end))).into())
    }

    pub(crate) fn contains(&self, char_index: &CharIndex) -> bool {
        &self.start <= char_index && char_index <= &self.end
    }

    pub(crate) fn trimmed(&self, buffer: &crate::buffer::Buffer) -> anyhow::Result<Self> {
        let text = buffer.slice(self)?.to_string();

        if text.chars().all(char::is_whitespace) {
            Ok(*self)
        } else {
            let leading_whitespace_count = text.chars().take_while(|c| c.is_whitespace()).count();
            let trailing_whitespace_count =
                text.chars().rev().take_while(|c| c.is_whitespace()).count();
            Ok(
                (self.start + leading_whitespace_count..self.end - trailing_whitespace_count)
                    .into(),
            )
        }
    }

    pub(crate) fn is_supserset_of(&self, other: &CharIndexRange) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub(crate) fn as_char_index(
        &self,
        cursor_direction: &crate::components::editor::Direction,
    ) -> CharIndex {
        match cursor_direction {
            crate::components::editor::Direction::Start => self.start,
            crate::components::editor::Direction::End => self.end - 1,
        }
    }
}

pub(crate) struct CharIndexRangeIter {
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
pub(crate) fn apply_edit(
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
