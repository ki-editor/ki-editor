pub mod ast_grep;
pub mod bookmark;
pub mod custom;
pub mod diagnostic;
pub mod git_hunk;
pub mod line;
pub mod outermost_node;
pub mod regex;
pub mod small_word;
pub mod syntax_tree;
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
pub use small_word::SmallWord;
use std::ops::Range;
pub use syntax_tree::SyntaxTree;
pub use token::Token;

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::editor::{CursorDirection, Jump, Movement},
    context::Context,
    edit::is_overlapping,
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

#[derive(Clone)]
pub struct SelectionModeParams<'a> {
    pub buffer: &'a Buffer,
    pub current_selection: &'a Selection,
    pub cursor_direction: &'a CursorDirection,
    pub context: &'a Context,
}

pub trait SelectionMode {
    fn name(&self) -> &'static str;
    fn iter<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
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
            Movement::Up => self.up(params),
            Movement::Down => self.down(params),
        }
    }

    fn up(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.select_vertical(params, std::cmp::Ordering::Less)
    }

    fn down(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.select_vertical(params, std::cmp::Ordering::Greater)
    }

    fn select_vertical(
        &self,
        params: SelectionModeParams,
        ordering: std::cmp::Ordering,
    ) -> anyhow::Result<Option<Selection>> {
        let SelectionModeParams {
            buffer,
            current_selection,
            ..
        } = params;
        let start = current_selection.range().start;
        let current_position = buffer.char_to_position(start)?;
        let current_line = buffer.char_to_line(start)?;
        self.iter(params)?
            .filter_map(|range| {
                let position = buffer.byte_to_position(range.range.start).ok()?;
                Some((position, range))
            })
            .filter(|(position, _)| position.line.cmp(&current_line) == ordering)
            .sorted_by_key(|(position, _)| {
                (
                    current_line.abs_diff(position.line),
                    position.column.abs_diff(current_position.column),
                )
            })
            .next()
            .map(|(_, range)| range.to_selection(buffer, current_selection))
            .transpose()
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
            .iter(params.clone())?
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
            .iter(params.clone())?
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
        let current_selection = params.current_selection;
        let buffer = params.buffer;
        let mut iter = self.iter(params)?.sorted();
        if let Some(byte_range) = iter.nth(index) {
            Ok(Some(byte_range.to_selection(buffer, current_selection)?))
        } else {
            Err(anyhow::anyhow!("Invalid index"))
        }
    }

    fn get_by_offset_to_current_selection(
        &self,
        params: SelectionModeParams,
        offset: isize,
    ) -> anyhow::Result<Option<Selection>> {
        let iter = self.iter(params.clone())?.sorted();
        let buffer = params.buffer;
        let current_selection = params.current_selection;

        // Find the range from the iterator that is most similar to the range of current selection
        let current_selection_range = current_selection.range();
        let current_selection_line = current_selection_range.start.to_line(params.buffer)?;

        let byte_range = buffer.char_to_byte(current_selection_range.start)?
            ..buffer.char_to_byte(current_selection_range.end)?;
        let info = current_selection.info();

        let nearest = iter
            .enumerate()
            .map(|(i, range)| {
                let line = buffer.byte_to_line(range.range.start).unwrap_or(0);
                (
                    i,
                    (
                        // NOTE: we use ! (not) because false ranks lower than true
                        // Prioritize selection of the same range
                        !(range.range == byte_range && range.info == info),
                        // Then by if they overlaps
                        !(is_overlapping(&range.range, &byte_range)),
                        // Then by selection that is one the same line
                        line.abs_diff(current_selection_line),
                        // Then by their distance to the current selection
                        range.range.start.abs_diff(byte_range.start),
                        // Then by their length
                        range.range.len(),
                    ),
                )
            })
            .min_by_key(|(_, diff)| *diff)
            .map(|(i, _)| i);

        let mut iter = self.iter(params)?.sorted();
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
            .iter(params.clone())?
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
            .iter(SelectionModeParams {
                buffer,
                current_selection: &current_selection,
                cursor_direction: &CursorDirection::Start,
                context: &Context::default(),
            })
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
        context::Context,
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
            _: super::SelectionModeParams<'a>,
        ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
            Ok(Box::new(
                [(0..6), (1..6), (2..5), (3..4), (3..5)]
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
            context: &Context::default(),
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
        test(Movement::Previous, 3..5, 3..4);
        test(Movement::Previous, 3..4, 2..5);
    }

    #[test]
    fn next() {
        test(Movement::Next, 0..6, 1..6);
        test(Movement::Next, 1..6, 2..5);
        test(Movement::Next, 2..5, 3..4);
        test(Movement::Next, 3..4, 3..5);
    }

    #[test]
    fn first() {
        test(Movement::First, 0..1, 0..6);
    }

    #[test]
    fn last() {
        test(Movement::Last, 0..0, 3..5);
    }

    #[test]
    fn current() {
        test(Movement::Current, 0..1, 0..6);
        test(Movement::Current, 5..6, 1..6);
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
    fn same_range_different_info() {
        let params = SelectionModeParams {
            context: &Context::default(),
            buffer: &Buffer::new(tree_sitter_md::language(), "hello world"),
            current_selection: &Selection::default()
                .set_range((CharIndex(1)..CharIndex(2)).into())
                .set_info(Some("Spongebob".to_string())),
            cursor_direction: &CursorDirection::Start,
        };
        struct Dummy;
        impl SelectionMode for Dummy {
            fn name(&self) -> &'static str {
                "dummy"
            }
            fn iter<'a>(
                &'a self,
                _: super::SelectionModeParams<'a>,
            ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
                Ok(Box::new(
                    [
                        ByteRange::with_info(1..2, "Spongebob".to_string()),
                        ByteRange::with_info(1..2, "Squarepants".to_string()),
                    ]
                    .into_iter(),
                ))
            }
        }
        let run_test = |movement: Movement, expected_info: &str| {
            let actual = Dummy
                .apply_direction(params.clone(), movement)
                .unwrap()
                .unwrap();
            let expected_range: CharIndexRange = (CharIndex(1)..CharIndex(2)).into();
            assert_eq!(expected_range, actual.range());
            let expected_info = expected_info.to_string();
            assert_eq!(expected_info, actual.info().unwrap());
        };
        run_test(Movement::Current, "Spongebob");
        run_test(Movement::Next, "Squarepants");
    }

    #[test]
    fn should_not_use_extended_range() {
        let params = SelectionModeParams {
            context: &Context::default(),
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
        let expected: CharIndexRange = (CharIndex(3)..CharIndex(4)).into();

        assert_eq!(expected, actual);
    }

    #[test]
    /// Should prioritize selection on the same line even though the selection on the next line might be closer to the cursor
    fn prioritize_same_line() {
        let params = SelectionModeParams {
            context: &Context::default(),
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
        let expected: CharIndexRange = (CharIndex(0)..CharIndex(5)).into();

        assert_eq!(expected, actual);
    }
}
