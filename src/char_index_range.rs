use std::ops::Range;

use crate::{edit::Edit, selection::CharIndex};

#[derive(
    PartialEq,
    Clone,
    Debug,
    Eq,
    Hash,
    Default,
    Copy,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
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

    pub(crate) fn as_usize_range(&self) -> Range<usize> {
        self.start.0..self.end.0
    }

    /// Range with 0 length (e.g. 1..1) is defined as never intersecting with other ranges.
    /// This is because a 0 length Edit represents a pure insertion without modifications,
    /// and multiple insertions at the same position are both theoretically feasible and practical.
    pub(crate) fn intersects_with(&self, other: &CharIndexRange) -> bool {
        self.len() > 0
            && other.len() > 0
            && range_intersects(&self.as_usize_range(), &other.as_usize_range())
    }

    pub(crate) fn subtracts(&self, other: &CharIndexRange) -> CharIndexRange {
        // If no intersection, return the original range
        if !self.intersects_with(other) {
            return *self;
        }

        // If other completely covers self
        if other.is_supserset_of(self) {
            // Return empty range at the start position
            return (self.start..self.start).into();
        }

        // If other overlaps the start of self
        if other.start <= self.start && other.end < self.end {
            // Return the portion after other ends
            return (other.end..self.end).into();
        }

        // If other overlaps the end of self
        if other.start > self.start && other.end >= self.end {
            // Return the portion before other starts
            return (self.start..other.start).into();
        }

        // If other is completely inside self (would split into two ranges)
        // Since we can only return one range, return the larger portion
        let left_size = other.start.0 - self.start.0;
        let right_size = self.end.0 - other.end.0;

        if left_size >= right_size {
            (self.start..other.start).into()
        } else {
            (other.end..self.end).into()
        }
    }
}

pub(crate) fn range_intersects<T: PartialOrd>(a: &Range<T>, b: &Range<T>) -> bool {
    a.start < b.end && b.start < a.end
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

#[cfg(test)]
mod test_subtract_range {
    use super::*;

    #[test]
    fn subtracts_no_overlap() {
        // [a, b) and [c, d) where b <= c
        let range1: CharIndexRange = (CharIndex(0)..CharIndex(5)).into();
        let range2 = (CharIndex(10)..CharIndex(15)).into();
        assert_eq!(range1.subtracts(&range2), range1);
        assert_eq!(range2.subtracts(&range1), range2);
    }

    #[test]
    fn subtracts_adjacent() {
        // [a, b) and [b, c) - touching but not overlapping
        let range1: CharIndexRange = (CharIndex(0)..CharIndex(5)).into();
        let range2 = (CharIndex(5)..CharIndex(10)).into();
        assert_eq!(range1.subtracts(&range2), range1);
        assert_eq!(range2.subtracts(&range1), range2);
    }

    #[test]
    fn subtracts_partial_overlap_left() {
        // [a, b) overlaps [c, d) where a < c < b < d
        let range1: CharIndexRange = (CharIndex(0)..CharIndex(10)).into();
        let range2 = (CharIndex(5)..CharIndex(15)).into();
        assert_eq!(
            range1.subtracts(&range2),
            (CharIndex(0)..CharIndex(5)).into()
        );
    }

    #[test]
    fn subtracts_partial_overlap_right() {
        // [a, b) overlaps [c, d) where c < a < d < b
        let range1: CharIndexRange = (CharIndex(5)..CharIndex(15)).into();
        let range2 = (CharIndex(0)..CharIndex(10)).into();
        assert_eq!(
            range1.subtracts(&range2),
            (CharIndex(10)..CharIndex(15)).into()
        );
    }

    #[test]
    fn subtracts_contained_within() {
        // [a, b) contains [c, d) where a < c < d < b
        // Returns the larger portion (left in this case)
        let range1: CharIndexRange = (CharIndex(0)..CharIndex(20)).into();
        let range2 = (CharIndex(5)..CharIndex(15)).into();
        assert_eq!(
            range1.subtracts(&range2),
            (CharIndex(0)..CharIndex(5)).into()
        );
    }

    #[test]
    fn subtracts_contained_within_right_larger() {
        // When right portion is larger, return right portion
        let range1: CharIndexRange = (CharIndex(0)..CharIndex(20)).into();
        let range2 = (CharIndex(5)..CharIndex(8)).into();
        assert_eq!(
            range1.subtracts(&range2),
            (CharIndex(8)..CharIndex(20)).into()
        );
    }

    #[test]
    fn subtracts_completely_contained_by() {
        // [a, b) is contained by [c, d) where c <= a < b <= d
        let range1: CharIndexRange = (CharIndex(5)..CharIndex(15)).into();
        let range2 = (CharIndex(0)..CharIndex(20)).into();
        assert_eq!(
            range1.subtracts(&range2),
            (CharIndex(5)..CharIndex(5)).into()
        );
    }

    #[test]
    fn subtracts_exact_match() {
        // [a, b) == [c, d)
        let range1: CharIndexRange = (CharIndex(5)..CharIndex(15)).into();
        let range2 = (CharIndex(5)..CharIndex(15)).into();
        assert_eq!(
            range1.subtracts(&range2),
            (CharIndex(5)..CharIndex(5)).into()
        );
    }

    #[test]
    fn subtracts_zero_length_ranges() {
        // Empty ranges
        let range1: CharIndexRange = (CharIndex(5)..CharIndex(5)).into();
        let range2 = (CharIndex(10)..CharIndex(10)).into();
        assert_eq!(range1.subtracts(&range2), range1);

        // Non-empty minus empty at same position
        let range1: CharIndexRange = (CharIndex(5)..CharIndex(10)).into();
        let range2 = (CharIndex(5)..CharIndex(5)).into();
        assert_eq!(range1.subtracts(&range2), range1);

        // Non-empty minus empty at end
        let range1: CharIndexRange = (CharIndex(5)..CharIndex(10)).into();
        let range2 = (CharIndex(10)..CharIndex(10)).into();
        assert_eq!(range1.subtracts(&range2), range1);
    }

    #[test]
    fn subtracts_subtrahend_extends_left() {
        // [c, d) extends beyond left of [a, b) where c < a < d <= b
        let range1: CharIndexRange = (CharIndex(5)..CharIndex(15)).into();
        let range2 = (CharIndex(0)..CharIndex(5)).into();
        assert_eq!(range1.subtracts(&range2), range1);

        let range2 = (CharIndex(0)..CharIndex(10)).into();
        assert_eq!(
            range1.subtracts(&range2),
            (CharIndex(10)..CharIndex(15)).into()
        );
    }

    #[test]
    fn subtracts_subtrahend_extends_right() {
        // [c, d) extends beyond right of [a, b) where a <= c < b < d
        let range1: CharIndexRange = (CharIndex(5)..CharIndex(15)).into();
        let range2 = (CharIndex(15)..CharIndex(20)).into();
        assert_eq!(range1.subtracts(&range2), range1);

        let range2 = (CharIndex(10)..CharIndex(20)).into();
        assert_eq!(
            range1.subtracts(&range2),
            (CharIndex(5)..CharIndex(10)).into()
        );
    }
}
