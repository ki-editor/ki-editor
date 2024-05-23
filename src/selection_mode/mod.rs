pub(crate) mod ast_grep;
pub(crate) mod bookmark;
pub(crate) mod case_agnostic;
pub(crate) mod column;
pub(crate) mod custom;
pub(crate) mod diagnostic;
pub(crate) mod git_hunk;
pub(crate) mod token;

pub(crate) mod line_full;
pub(crate) mod line_trimmed;
pub(crate) mod local_quickfix;
pub(crate) mod regex;
pub(crate) mod syntax_tree;
pub(crate) mod top_node;
pub(crate) mod word_long;
pub(crate) mod word_short;
pub(crate) use self::regex::Regex;
pub(crate) use ast_grep::AstGrep;
pub(crate) use bookmark::Bookmark;
pub(crate) use case_agnostic::CaseAgnostic;
pub(crate) use column::Column;
pub(crate) use custom::Custom;
pub(crate) use diagnostic::Diagnostic;
pub(crate) use git_hunk::GitHunk;
use itertools::Itertools;
pub(crate) use line_full::LineFull;
pub(crate) use line_trimmed::LineTrimmed;
pub(crate) use local_quickfix::LocalQuickfix;
use std::ops::Range;
pub(crate) use syntax_tree::SyntaxTree;
pub(crate) use token::Token;
pub(crate) use top_node::TopNode;
pub(crate) use word_long::WordLong;
pub(crate) use word_short::WordShort;

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        editor::{Direction, Jump, Movement},
        suggestive_editor::Info,
    },
    edit::is_overlapping,
    selection::{Filters, Selection},
};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub(crate) struct ByteRange {
    range: Range<usize>,
    info: Option<Info>,
}
impl ByteRange {
    pub(crate) fn new(range: Range<usize>) -> Self {
        Self { range, info: None }
    }

    pub(crate) fn with_info(range: Range<usize>, info: Info) -> Self {
        Self {
            range,
            info: Some(info),
        }
    }
    pub(crate) fn to_char_index_range(&self, buffer: &Buffer) -> anyhow::Result<CharIndexRange> {
        Ok((buffer.byte_to_char(self.range.start)?..buffer.byte_to_char(self.range.end)?).into())
    }

    pub(crate) fn to_selection(
        &self,
        buffer: &Buffer,
        selection: &Selection,
    ) -> anyhow::Result<Selection> {
        Ok(selection
            .clone()
            .set_range(self.to_char_index_range(buffer)?)
            .set_info(self.info.clone()))
    }

    fn set_info(self, info: Option<Info>) -> ByteRange {
        ByteRange { info, ..self }
    }

    pub(crate) fn range(&self) -> &Range<usize> {
        &self.range
    }

    pub(crate) fn info(&self) -> &Option<Info> {
        &self.info
    }
}

impl PartialOrd for ByteRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
pub(crate) struct SelectionModeParams<'a> {
    pub(crate) buffer: &'a Buffer,
    pub(crate) current_selection: &'a Selection,
    pub(crate) cursor_direction: &'a Direction,
    pub(crate) filters: &'a Filters,
}
#[derive(Debug)]
pub(crate) struct ApplyMovementResult {
    pub(crate) selection: Selection,
    pub(crate) mode: Option<crate::selection::SelectionMode>,
}

impl ApplyMovementResult {
    pub(crate) fn from_selection(selection: Selection) -> Self {
        Self {
            selection,
            mode: None,
        }
    }
}

pub trait SelectionMode {
    fn name(&self) -> &'static str;
    /// NOTE: this method should not be used directly,
    /// Use `iter_filtered` instead.
    /// I wish to have private trait methods :(
    fn iter<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>>;

    fn iter_filtered<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let SelectionModeParams {
            buffer, filters, ..
        } = params;

        Ok(Box::new(
            self.iter(params)?
                .filter_map(|item| filters.retain(buffer, item))
                .group_by(|item| item.range.clone())
                .into_iter()
                .map(|(range, items)| {
                    let infos = items.into_iter().filter_map(|item| item.info).collect_vec();
                    let info = infos.split_first().map(|(head, tail)| {
                        Info::new(
                            Some(head.title())
                                .into_iter()
                                .chain(tail.iter().map(|tail| tail.title()))
                                .unique()
                                .join(" & "),
                            Some(head.content())
                                .into_iter()
                                .chain(tail.iter().map(|tail| tail.content()))
                                .unique()
                                .join("\n=======\n"),
                        )
                        .set_decorations(head.decorations().clone())
                    });
                    ByteRange::new(range).set_info(info)
                })
                .collect_vec()
                .into_iter(),
        ))
    }

    fn apply_movement(
        &self,
        params: SelectionModeParams,
        movement: Movement,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        fn convert(
            result: anyhow::Result<Option<Selection>>,
        ) -> anyhow::Result<Option<ApplyMovementResult>> {
            Ok(result?.map(|result| result.into()))
        }
        match movement {
            Movement::Next => convert(self.next(params)),

            Movement::Previous => convert(self.previous(params)),
            Movement::Last => convert(self.last(params)),
            Movement::Current => convert(self.current(params)),
            Movement::First => convert(self.first(params)),
            Movement::Index(index) => convert(self.to_index(params, index)),
            Movement::Jump(range) => Ok(Some(ApplyMovementResult::from_selection(
                params.current_selection.clone().set_range(range),
            ))),
            Movement::Up => convert(self.up(params)),
            Movement::Down => convert(self.down(params)),
            Movement::ToParentLine => convert(self.to_parent_line(params)),
            Movement::Parent => self.parent(params),
            #[cfg(test)]
            Movement::FirstChild => self.first_child(params),
        }
    }

    fn parent(&self, _: SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        Ok(None)
    }

    fn first_child(&self, _: SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        Ok(None)
    }

    fn to_parent_line(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        let SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
            filters,
        } = params;
        let current_line = buffer.char_to_line(current_selection.extended_range().start)?;
        Ok(buffer
            .get_parent_lines(current_line)?
            .into_iter()
            .filter(|line| line.line < current_line)
            .next_back()
            .map(|line| {
                let byte_range = buffer.line_to_byte_range(line.line)?;
                let start =
                    line_trimmed::trim_leading_spaces(byte_range.range.start, &line.content);
                let char_index_range =
                    buffer.byte_range_to_char_index_range(&(start..start + 1))?;
                self.current(SelectionModeParams {
                    buffer,
                    cursor_direction,
                    current_selection: &current_selection.clone().set_range(char_index_range),
                    filters,
                })
            })
            .transpose()?
            .flatten())
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
        let selection = self
            .iter_filtered(params)?
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
            .transpose()?;
        Ok(selection)
    }

    fn selections_in_line_number_range(
        &self,
        params: &SelectionModeParams,
        line_number_range: Range<usize>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let byte_range = params.buffer.line_to_byte(line_number_range.start)?
            ..params.buffer.line_to_byte(line_number_range.end)?;
        Ok(self
            .iter_filtered(params.clone())?
            .filter(|range| (byte_range.start..byte_range.end).contains(&range.range.start))
            .collect_vec())
    }

    fn jumps(
        &self,
        params: SelectionModeParams,
        chars: Vec<char>,
        line_number_range: Range<usize>,
    ) -> anyhow::Result<Vec<Jump>> {
        let iter = self
            .selections_in_line_number_range(&params, line_number_range)?
            .into_iter();
        let jumps = iter
            .filter_map(|range| {
                let selection = range
                    .to_selection(params.buffer, params.current_selection)
                    .ok()?;
                let character = params
                    .buffer
                    .slice(&selection.range()) // Cannot use extend_range here, must use range only
                    .ok()?
                    .chars()
                    .next()?
                    .to_ascii_lowercase();
                Some(Jump {
                    character,
                    selection,
                })
            })
            .collect_vec();
        let jumps = if jumps
            .iter()
            .group_by(|jump| jump.character)
            .into_iter()
            .count()
            > 1
        {
            jumps
        } else {
            // All jumps has the same chars, assign their char using the given chars set
            chars
                .into_iter()
                .cycle()
                .zip(jumps)
                .map(|(char, jump)| Jump {
                    character: char,
                    selection: jump.selection,
                })
                .collect_vec()
        };
        Ok(jumps)
    }

    fn last(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .iter_filtered(params.clone())?
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
        let mut iter = self.iter_filtered(params)?.sorted();
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
        let iter = self.iter_filtered(params.clone())?.sorted();
        let buffer = params.buffer;
        let current_selection = params.current_selection;

        // Find the range from the iterator that is most similar to the range of current selection
        let current_selection_range = current_selection.range();
        let cursor_position = current_selection_range.cursor_position(params.cursor_direction);
        let current_selection_line = cursor_position.to_line(params.buffer)?;

        let byte_range = match params.cursor_direction {
            Direction::Start => {
                buffer.char_to_byte(current_selection_range.start)?
                    ..buffer.char_to_byte(current_selection_range.end)?
            }
            Direction::End => {
                let start = buffer.char_to_byte(current_selection_range.end - 1)?;
                start..(start + 1)
            }
        };
        let info = current_selection.info();

        let nearest = iter
            .enumerate()
            .map(|(i, range)| {
                let cursor_position = match params.cursor_direction {
                    Direction::Start => range.range.start,
                    Direction::End => range.range.end,
                };
                let line = buffer.byte_to_line(cursor_position).unwrap_or(0);
                (
                    i,
                    (
                        // NOTE: we use ! (not) because false ranks lower than true
                        // Prioritize selection of the same range
                        !(range.range == byte_range && range.info == info),
                        // Then by selection that is one the same line
                        line.abs_diff(current_selection_line),
                        // Then by if they overlaps
                        !(is_overlapping(&range.range, &byte_range)),
                        // Then by their distance to the current selection
                        cursor_position.abs_diff(byte_range.start),
                        // Then by their length
                        range.range.len(),
                    ),
                )
            })
            .min_by_key(|(_, diff)| *diff)
            .map(|(i, _)| i);

        let mut iter = self.iter_filtered(params)?.sorted();
        Ok(nearest.and_then(|i| {
            iter.nth(((i as isize) + offset) as usize)
                .and_then(|range| range.to_selection(buffer, current_selection).ok())
        }))
    }

    fn next(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        let current_selection = params.current_selection.clone();
        let buffer = params.buffer;
        let byte_range = buffer.char_index_range_to_byte_range(current_selection.range())?;
        Ok(self
            .iter_filtered(params)?
            .sorted()
            .find(|range| {
                range.range.start > byte_range.start
                    || (range.range.start == byte_range.start && range.range.end > byte_range.end)
            })
            .and_then(|range| range.to_selection(buffer, &current_selection).ok()))
    }

    fn previous(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        let current_selection = params.current_selection.clone();
        let buffer = params.buffer;
        let byte_range = buffer.char_index_range_to_byte_range(current_selection.range())?;

        Ok(self
            .iter_filtered(params)?
            .sorted()
            .rev()
            .find(|range| {
                range.range.start < byte_range.start
                    || (range.range.start == byte_range.start && range.range.end < byte_range.end)
            })
            .and_then(|range| range.to_selection(buffer, &current_selection).ok()))
    }

    fn first(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .iter_filtered(params.clone())?
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
                cursor_direction: &Direction::default(),
                filters: &Filters::default(),
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

    #[cfg(test)]
    fn generate_selections(
        &self,
        buffer: &Buffer,
        movement: Movement,
        up_to: usize,
        initial_range: CharIndexRange,
    ) -> anyhow::Result<Vec<String>> {
        let params = SelectionModeParams {
            buffer,
            current_selection: &Selection::default(),
            cursor_direction: &Direction::default(),
            filters: &Filters::default(),
        };
        Ok((0..up_to)
            .try_fold(
                (initial_range, Vec::new()),
                |result, _| -> anyhow::Result<_> {
                    let (range, mut results) = result;
                    let selection = self.apply_movement(
                        SelectionModeParams {
                            current_selection: &Selection::new(range),
                            ..params
                        },
                        movement,
                    );

                    let parent_range = selection.unwrap().unwrap().selection.range();
                    results.push(buffer.slice(&parent_range)?.to_string());
                    Ok((parent_range, results))
                },
            )?
            .1)
    }
}

#[cfg(test)]
mod test_selection_mode {
    use std::ops::Range;

    use crate::{
        buffer::Buffer,
        char_index_range::CharIndexRange,
        components::{
            editor::{Direction, Movement},
            suggestive_editor::Info,
        },
        selection::{CharIndex, Filters, Selection},
        selection_mode::LineTrimmed,
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
            buffer: &Buffer::new(None, "hello world"),
            current_selection: &Selection::default().set_range(CharIndexRange {
                start: CharIndex(current_selection_byte_range.start),
                end: CharIndex(current_selection_byte_range.end),
            }),
            cursor_direction: &Direction::default(),
            filters: &Filters::default(),
        };
        let actual = Dummy
            .apply_movement(params, movement)
            .unwrap()
            .unwrap()
            .selection
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
    fn same_range_different_info_should_be_merged() {
        let params = SelectionModeParams {
            buffer: &Buffer::new(None, "hello world"),
            current_selection: &Selection::default()
                .set_range((CharIndex(1)..CharIndex(2)).into())
                .set_info(Some(Info::new(
                    "Title".to_string(),
                    "Spongebob".to_string(),
                ))),
            cursor_direction: &Direction::default(),
            filters: &Filters::default(),
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
                        ByteRange::with_info(
                            1..2,
                            Info::new("Title".to_string(), "Spongebob".to_string()),
                        ),
                        ByteRange::with_info(
                            1..2,
                            Info::new("Title".to_string(), "Squarepants".to_string()),
                        ),
                    ]
                    .into_iter(),
                ))
            }
        }
        let run_test = |movement: Movement, expected_info: &str| {
            let actual = Dummy
                .apply_movement(params.clone(), movement)
                .unwrap()
                .unwrap()
                .selection;
            let expected_range: CharIndexRange = (CharIndex(1)..CharIndex(2)).into();
            assert_eq!(expected_range, actual.range());
            let expected_info = Info::new("Title".to_string(), expected_info.to_string());
            assert_eq!(expected_info, actual.info().unwrap());
        };
        run_test(Movement::Current, "Spongebob\n=======\nSquarepants");
    }

    #[test]
    fn should_not_use_extended_range() {
        let params = SelectionModeParams {
            buffer: &Buffer::new(None, "hello world"),
            current_selection: &Selection::default()
                .set_range(CharIndexRange {
                    start: CharIndex(2),
                    end: CharIndex(5),
                })
                .set_initial_range(Some(CharIndexRange {
                    start: CharIndex(0),
                    end: CharIndex(6),
                })),
            cursor_direction: &Direction::default(),
            filters: &Filters::default(),
        };
        let actual = Dummy
            .apply_movement(params, Movement::Next)
            .unwrap()
            .unwrap()
            .selection
            .range();
        let expected: CharIndexRange = (CharIndex(3)..CharIndex(4)).into();

        assert_eq!(expected, actual);
    }

    #[test]
    /// Should prioritize selection on the same line even though the selection on the next line might be closer to the cursor
    fn prioritize_same_line() {
        let params = SelectionModeParams {
            buffer: &Buffer::new(None, "hello\nworld"),
            current_selection: &Selection::default().set_range(CharIndexRange {
                start: CharIndex(4),
                end: CharIndex(5),
            }),
            cursor_direction: &Direction::default(),
            filters: &Filters::default(),
        };
        let actual = LineTrimmed
            .apply_movement(params, Movement::Current)
            .unwrap()
            .unwrap()
            .selection
            .range();
        let expected: CharIndexRange = (CharIndex(0)..CharIndex(5)).into();

        assert_eq!(expected, actual);
    }

    #[test]
    fn to_parent_line() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
            "
fn f() {
    fn g() {
        let a = 1;
        let b = 2;
        let c = 3;
        let d = 4;
    }

}"
            .trim(),
        );

        let test = |selected_line: usize, expected: &str| {
            let start = buffer.line_to_char(selected_line).unwrap();
            let result = LineTrimmed
                .to_parent_line(SelectionModeParams {
                    buffer: &buffer,
                    current_selection: &Selection::new((start..start + 1).into()),
                    cursor_direction: &Direction::default(),
                    filters: &Filters::default(),
                })
                .unwrap()
                .unwrap();

            let actual = buffer.slice(&result.extended_range()).unwrap();
            assert_eq!(actual, expected);
        };

        test(4, "fn g() {");

        test(1, "fn f() {");
    }
}
