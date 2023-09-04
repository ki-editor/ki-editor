pub mod ast_grep;
pub mod bookmark;
pub mod custom;
pub mod diagnostic;
pub mod git_hunk;
pub mod line;
pub mod outermost_node;
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
pub use line::Line;
pub use outermost_node::OutermostNode;
pub use sibling::Sibling;
pub use syntax_hierarchy::SyntaxHierarchy;
pub use token::Token;

use std::ops::Range;

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::editor::{CursorDirection, Jump, Movement},
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
                    .map(|ordering| ordering.reverse())
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
    fn name(&self) -> &'static str;
    fn iter<'a>(
        &'a self,
        current_selection: &'a Selection,
        buffer: &'a Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>>;

    fn apply_direction(
        &self,
        params: SelectionModeParams,
        movement: Movement,
    ) -> anyhow::Result<Option<Selection>> {
        match movement {
            Movement::Next => self.next(params),
            Movement::Previous => self.previous(params),
            Movement::Last => self.last(params),
            Movement::Current => self.current(params),
            Movement::First => self.first(params),
            Movement::Index(index) => self.to_index(params, index),
            Movement::Jump(range) => Ok(Some(params.current_selection.clone().set_range(range))),
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
            .filter(|range| (byte_range.start..byte_range.end).contains(&range.range.start));
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

    fn next(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_by_offset_to_current_selection(params, 1)
    }

    fn last(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
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
    fn to_index(
        &self,
        params: SelectionModeParams,
        index: usize,
    ) -> anyhow::Result<Option<Selection>> {
        let mut iter = self.iter(params.current_selection, params.buffer)?.sorted();
        if let Some(byte_range) = iter.nth(index) {
            Ok(Some(
                byte_range.to_selection(params.buffer, params.current_selection)?,
            ))
        } else {
            Err(anyhow::anyhow!("Invalid index"))
        }
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
        let current_selection_range = current_selection.range();
        let current_selection_line = current_selection_range.start.to_line(params.buffer)?;

        let byte_range = buffer.char_to_byte(current_selection_range.start)?
            ..buffer.char_to_byte(current_selection_range.end)?;

        let nearest = iter
            .enumerate()
            .map(|(i, range)| {
                let line = buffer.byte_to_line(range.range.start).unwrap_or(0);
                (
                    i,
                    // Prioritize selection that is one the same line
                    (
                        line.abs_diff(current_selection_line),
                        range.range.start.abs_diff(byte_range.start),
                        (range.range.end.abs_diff(byte_range.end)),
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

    fn previous(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_by_offset_to_current_selection(params, -1)
    }

    fn first(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .iter(params.current_selection, params.buffer)?
            .sorted()
            .next()
            .and_then(|range| {
                range
                    .to_selection(params.buffer, params.current_selection)
                    .ok()
            }))
    }

    fn current(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_by_offset_to_current_selection(params, 0)
    }

    #[cfg(test)]
    fn assert_all_selections(
        &self,
        buffer: &Buffer,
        current_selection: Selection,
        selections: &[(Range<usize>, &'static str)],
    ) {
        let expected = selections
            .iter()
            .map(|(range, info)| (range.to_owned(), info.to_string()))
            .collect_vec();

        let actual = self
            .iter(&current_selection, buffer)
            .unwrap()
            .flat_map(|range| -> anyhow::Result<_> {
                Ok((
                    range.range.start..range.range.end,
                    buffer
                        .slice(&range.to_char_index_range(buffer)?)?
                        .to_string(),
                ))
            })
            .collect_vec();

        assert_eq!(expected, actual);
    }
}

#[cfg(test)]
mod test_selection_mode {
    use std::ops::Range;

    use crate::{
        buffer::Buffer,
        char_index_range::CharIndexRange,
        components::editor::{CursorDirection, Movement},
        selection::{CharIndex, Selection},
        selection_mode::Line,
    };

    use super::{ByteRange, SelectionMode, SelectionModeParams};
    use pretty_assertions::assert_eq;

    struct Dummy;
    impl SelectionMode for Dummy {
        fn name(&self) -> &'static str {
            "dummy"
        }
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
        movement: Movement,
        current_selection_byte_range: Range<usize>,
        expected_selection_byte_range: Range<usize>,
    ) {
        let params = SelectionModeParams {
            buffer: &Buffer::new(tree_sitter_md::language(), "hello world"),
            current_selection: &Selection::default().set_range(CharIndexRange {
                start: CharIndex(current_selection_byte_range.start),
                end: CharIndex(current_selection_byte_range.end),
            }),
            cursor_direction: &CursorDirection::Start,
        };
        let actual = Dummy
            .apply_direction(params, movement)
            .unwrap()
            .unwrap()
            .range();
        let expected: CharIndexRange = (CharIndex(expected_selection_byte_range.start)
            ..CharIndex(expected_selection_byte_range.end))
            .into();

        assert_eq!(expected, actual);
    }

    #[test]
    fn previous() {
        test(Movement::Previous, 1..6, 0..6);
        test(Movement::Previous, 2..5, 1..6);

        // Ranges is expected to be sorted by start ascendingly, and end descendingly
        test(Movement::Previous, 3..4, 3..5);
        test(Movement::Previous, 3..5, 2..5);
    }

    #[test]
    fn next() {
        test(Movement::Next, 0..6, 1..6);
        test(Movement::Next, 1..6, 2..5);
        test(Movement::Next, 2..5, 3..5);
        test(Movement::Next, 3..5, 3..4);
    }

    #[test]
    fn first() {
        test(Movement::First, 0..1, 0..6);
    }

    #[test]
    fn last() {
        test(Movement::Last, 0..0, 3..4);
    }

    #[test]
    fn current() {
        test(Movement::Current, 0..1, 0..6);
        test(Movement::Current, 1..2, 1..6);
        test(Movement::Current, 3..3, 3..4);
    }

    #[test]
    fn to_index() {
        let current = 0..0;
        test(Movement::Index(0), current.clone(), 0..6);
        test(Movement::Index(1), current, 1..6)
    }

    #[test]
    fn should_not_use_extended_range() {
        let params = SelectionModeParams {
            buffer: &Buffer::new(tree_sitter_md::language(), "hello world"),
            current_selection: &Selection::default()
                .set_range(CharIndexRange {
                    start: CharIndex(2),
                    end: CharIndex(5),
                })
                .set_initial_range(Some(CharIndexRange {
                    start: CharIndex(0),
                    end: CharIndex(6),
                })),
            cursor_direction: &CursorDirection::Start,
        };
        let actual = Dummy
            .apply_direction(params, Movement::Next)
            .unwrap()
            .unwrap()
            .range();
        let expected: CharIndexRange = (CharIndex(3)..CharIndex(5)).into();

        assert_eq!(expected, actual);
    }

    #[test]
    /// Should prioritize selection on the same line even though the selection on the next line might be closer to the cursor
    fn prioritize_same_line() {
        let params = SelectionModeParams {
            buffer: &Buffer::new(tree_sitter_md::language(), "hello\nworld"),
            current_selection: &Selection::default().set_range(CharIndexRange {
                start: CharIndex(4),
                end: CharIndex(5),
            }),
            cursor_direction: &CursorDirection::Start,
        };
        let actual = Line
            .apply_direction(params, Movement::Current)
            .unwrap()
            .unwrap()
            .range();
        let expected: CharIndexRange = (CharIndex(0)..CharIndex(6)).into();

        assert_eq!(expected, actual);
    }
}
