pub mod ast_grep;
pub mod bookmark;
pub mod custom;
pub mod diagnostic;
pub mod git_hunk;
pub mod largest_node;
pub mod line;
pub mod regex;
pub mod sibling;
pub mod syntax_hierarchy;
pub mod token;

pub use self::regex::Regex;
pub use ast_grep::AstGrep;
pub use bookmark::Bookmark;
pub use custom::Custom;
pub use diagnostic::Diagnostic;
pub use git_hunk::GitHunk;
use itertools::Itertools;
pub use largest_node::LargestNode;
pub use line::Line;
pub use sibling::Sibling;
pub use syntax_hierarchy::SyntaxHierarchy;
pub use token::Token;

use std::ops::Range;

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::editor::{CursorDirection, Direction, Jump},
    position::Position,
    selection::Selection,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ByteRange {
    range: Range<usize>,
    info: Option<String>,
}
impl ByteRange {
    pub fn new(range: Range<usize>) -> Self {
        Self { range, info: None }
    }

    pub fn with_info(range: Range<usize>, info: String) -> Self {
        Self {
            range,
            info: Some(info),
        }
    }
    pub fn to_char_index_range(&self, buffer: &Buffer) -> anyhow::Result<CharIndexRange> {
        Ok((buffer.byte_to_char(self.range.start)?..buffer.byte_to_char(self.range.end)?).into())
    }

    fn to_byte(&self, cursor_direction: &CursorDirection) -> usize {
        match cursor_direction {
            CursorDirection::Start => self.range.start,
            CursorDirection::End => self.range.end,
        }
    }

    pub fn to_selection(self, buffer: &Buffer, selection: &Selection) -> anyhow::Result<Selection> {
        Ok(selection
            .clone()
            .set_range(self.to_char_index_range(buffer)?)
            .set_info(self.info))
    }
}

impl PartialOrd for ByteRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.range
            .start
            .partial_cmp(&other.range.start)
            .or_else(|| {
                self.range
                    .end
                    .partial_cmp(&other.range.end)
                    .and_then(|ordering| Some(ordering.reverse()))
            })
    }
}

impl Ord for ByteRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.range
            .start
            .cmp(&other.range.start)
            .then(self.range.end.cmp(&other.range.end))
    }
}

pub struct SelectionModeParams<'a> {
    pub buffer: &'a Buffer,
    pub current_selection: &'a Selection,
    pub cursor_direction: &'a CursorDirection,
}

pub trait SelectionMode {
    fn iter<'a>(
        &'a self,
        current_selection: &'a Selection,
        buffer: &'a Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>>;

    fn apply_direction(
        &self,
        params: SelectionModeParams,
        direction: Direction,
    ) -> anyhow::Result<Option<Selection>> {
        match direction {
            Direction::Right => self.right(params),
            Direction::Left => self.left(params),
            Direction::RightMost => self.right_most(params),
            Direction::Current => self.current(params),
            Direction::LeftMost => self.left_most(params),
        }
    }

    fn jumps(
        &self,
        params: SelectionModeParams,
        chars: Vec<char>,
        line_number_range: Range<usize>,
    ) -> anyhow::Result<Vec<Jump>> {
        let byte_range = params.buffer.line_to_byte(line_number_range.start)?
            ..params.buffer.line_to_byte(line_number_range.end)?;
        let iter = self
            .iter(params.current_selection, params.buffer)?
            .filter(|range| {
                byte_range.start <= range.range.start && range.range.end <= byte_range.end
            });
        Ok(chars
            .into_iter()
            .cycle()
            .zip(iter)
            .filter_map(|(character, range)| {
                Some(Jump {
                    character,
                    selection: range
                        .to_selection(params.buffer, params.current_selection)
                        .ok()?,
                })
            })
            .collect_vec())
    }

    fn right(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_by_offset_to_current_selection(params, 1)
    }

    fn right_most(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .iter(params.current_selection, params.buffer)?
            .sorted()
            .last()
            .and_then(|range| {
                range
                    .to_selection(params.buffer, params.current_selection)
                    .ok()
            }))
    }

    fn get_by_offset_to_current_selection(
        &self,
        params: SelectionModeParams,
        offset: isize,
    ) -> anyhow::Result<Option<Selection>> {
        let iter = self.iter(params.current_selection, params.buffer)?.sorted();
        let buffer = params.buffer;
        let current_selection = params.current_selection;

        // Find the range from the iterator that is most similar to the range of current selection
        let byte_range = buffer.char_to_byte(current_selection.extended_range().start)?
            ..buffer.char_to_byte(current_selection.extended_range().end)?;

        let nearest = iter
            .enumerate()
            .map(|(i, range)| {
                (
                    i,
                    (
                        range.range.start.abs_diff(byte_range.start),
                        range.range.end.abs_diff(byte_range.end),
                    ),
                )
            })
            .min_by_key(|(_, diff)| *diff)
            .map(|(i, _)| i);

        let mut iter = self.iter(params.current_selection, params.buffer)?.sorted();
        Ok(nearest.and_then(|i| {
            iter.nth(((i as isize) + offset) as usize)
                .and_then(|range| range.to_selection(buffer, current_selection).ok())
        }))
    }

    fn left(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        return self.get_by_offset_to_current_selection(params, -1);
    }

    fn left_most(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .iter(&params.current_selection, params.buffer)?
            .sorted()
            .next()
            .and_then(|range| {
                range
                    .to_selection(params.buffer, params.current_selection)
                    .ok()
            }))
    }

    fn current(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        return self.get_by_offset_to_current_selection(params, 0);
    }

    #[cfg(test)]
    fn assert_all_selections(
        &self,
        buffer: &Buffer,
        current_selection: Selection,
        selections: &[(Range<usize>, &'static str)],
    ) {
        let expected = selections
            .into_iter()
            .map(|(range, info)| (range.to_owned(), info.to_string()))
            .collect_vec();

        let actual = self
            .iter(&current_selection, &buffer)
            .unwrap()
            .map(|range| -> anyhow::Result<_> {
                Ok((
                    range.range.start..range.range.end,
                    buffer
                        .slice(&range.to_char_index_range(&buffer)?)?
                        .to_string(),
                ))
            })
            .flatten()
            .collect_vec();

        pretty_assertions::assert_eq!(expected, actual);
    }
}

#[cfg(test)]
mod test_selection_mode {
    use std::ops::Range;

    use crate::{
        buffer::Buffer,
        char_index_range::CharIndexRange,
        components::editor::Direction,
        selection::{CharIndex, Selection},
    };

    use super::{ByteRange, SelectionMode, SelectionModeParams};
    use pretty_assertions::assert_eq;

    struct Dummy;
    impl SelectionMode for Dummy {
        fn iter<'a>(
            &'a self,
            _: &'a crate::selection::Selection,
            _: &'a crate::buffer::Buffer,
        ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
            Ok(Box::new(
                [(0..6), (1..6), (2..5), (3..5), (3..4)]
                    .into_iter()
                    .map(ByteRange::new),
            ))
        }
    }

    fn test(
        direction: Direction,
        current_selection_byte_range: Range<usize>,
        expected_selection_byte_range: Range<usize>,
    ) {
        let params = SelectionModeParams {
            buffer: &Buffer::new(tree_sitter_md::language(), "hello world"),
            current_selection: &Selection::default().set_range(CharIndexRange {
                start: CharIndex(current_selection_byte_range.start),
                end: CharIndex(current_selection_byte_range.end),
            }),
            cursor_direction: &crate::components::editor::CursorDirection::Start,
        };
        let actual = Dummy
            .apply_direction(params, direction)
            .unwrap()
            .unwrap()
            .range();
        let expected: CharIndexRange = (CharIndex(expected_selection_byte_range.start)
            ..CharIndex(expected_selection_byte_range.end))
            .into();

        assert_eq!(expected, actual);
    }

    #[test]
    fn left() {
        test(Direction::Left, 1..6, 0..6);
        test(Direction::Left, 2..5, 1..6);

        // Ranges is expected to be sorted by start ascendingly, and end descendingly
        test(Direction::Left, 3..4, 3..5);
        test(Direction::Left, 3..5, 2..5);
    }

    #[test]
    fn right() {
        test(Direction::Right, 0..6, 1..6);
        test(Direction::Right, 1..6, 2..5);
        test(Direction::Right, 2..5, 3..5);
        test(Direction::Right, 3..5, 3..4);
    }

    #[test]
    fn left_most() {
        test(Direction::LeftMost, 0..1, 0..6);
    }

    #[test]
    fn right_most() {
        test(Direction::RightMost, 0..0, 3..4);
    }

    #[test]
    fn current() {
        test(Direction::Current, 0..1, 0..6);
        test(Direction::Current, 1..2, 1..6);
        test(Direction::Current, 3..3, 3..4);
    }
}
