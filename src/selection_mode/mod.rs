pub(crate) mod ast_grep;
pub(crate) mod case_agnostic;
pub(crate) mod column;
pub(crate) mod custom;
pub(crate) mod diagnostic;
pub(crate) mod git_hunk;
pub(crate) mod mark;
#[cfg(test)]
pub(crate) mod token;

pub(crate) mod line_full;
pub(crate) mod line_trimmed;
pub(crate) mod local_quickfix;
pub(crate) mod regex;
pub(crate) mod syntax_node;
pub(crate) mod top_node;
pub(crate) mod word_long;
pub(crate) mod word_short;
pub(crate) use self::regex::Regex;
pub(crate) use ast_grep::AstGrep;
pub(crate) use case_agnostic::CaseAgnostic;
pub(crate) use column::Column;
pub(crate) use custom::Custom;
pub(crate) use diagnostic::Diagnostic;
pub(crate) use git_hunk::GitHunk;
use itertools::Itertools;
pub(crate) use line_full::LineFull;
pub(crate) use line_trimmed::LineTrimmed;
pub(crate) use local_quickfix::LocalQuickfix;
pub(crate) use mark::Mark;
use std::ops::Range;
pub(crate) use syntax_node::SyntaxNode;
#[cfg(test)]
pub(crate) use token::Token;
pub(crate) use top_node::TopNode;
pub(crate) use word_long::WordLong;
pub(crate) use word_short::WordShort;

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        editor::{Direction, IfCurrentNotFound, Jump, Movement},
        suggestive_editor::Info,
    },
    selection::Selection,
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
}
#[derive(Debug, Clone)]
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
        Ok(Box::new(
            self.iter(params)?
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

    fn all_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        self.iter_filtered(params)
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
            Movement::Current(if_current_not_found) => {
                convert(self.current(params, if_current_not_found))
            }
            Movement::First => convert(self.first(params)),
            Movement::Index(index) => convert(self.to_index(params, index)),
            Movement::Jump(range) => Ok(Some(ApplyMovementResult::from_selection(
                params.current_selection.clone().set_range(range),
            ))),
            Movement::Up => convert(self.up(params)),
            Movement::Down => convert(self.down(params)),
            Movement::ToParentLine => convert(self.to_parent_line(params)),
            Movement::Parent => self.parent(params),
            Movement::FirstChild => self.first_child(params),
            Movement::RealNext => convert(self.real_next(params)),
            Movement::RealPrevious => convert(self.real_previous(params)),
        }
    }

    fn parent(&self, _: SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        Ok(None)
    }

    fn first_child(&self, _: SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        Ok(None)
    }

    fn real_next(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.next(params)
    }

    fn real_previous(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.previous(params)
    }

    fn to_parent_line(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        let SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
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
                self.current(
                    SelectionModeParams {
                        buffer,
                        cursor_direction,
                        current_selection: &current_selection.clone().set_range(char_index_range),
                    },
                    IfCurrentNotFound::LookForward,
                )
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
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let byte_ranges: Vec<_> = line_number_ranges
            .into_iter()
            .filter_map(|range| {
                Some(
                    params.buffer.line_to_byte(range.start).ok()?
                        ..params.buffer.line_to_byte(range.end).ok()?,
                )
            })
            .collect();

        Ok(self
            .iter_filtered(params.clone())?
            .filter(|range| {
                byte_ranges
                    .iter()
                    .any(|byte_range| byte_range.contains(&range.range.start))
            })
            .collect())
    }

    fn jumps(
        &self,
        params: SelectionModeParams,
        chars: Vec<char>,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<Jump>> {
        let iter = self
            .selections_in_line_number_range(&params, line_number_ranges)?
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

    fn current(
        &self,
        params: SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.current_default_impl(params, if_current_not_found)
    }

    fn current_default_impl(
        &self,
        params: SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<Selection>> {
        let current_selection = params.current_selection;
        let buffer = params.buffer;
        if let Some((_, best_intersecting_match)) = {
            let cursor_char_index = current_selection.to_char_index(params.cursor_direction);
            let cursor_line = buffer.char_to_line(cursor_char_index)?;
            let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
            self.iter_filtered(params.clone())?
                .filter_map(|byte_range| {
                    // Get intersecting matches
                    if byte_range.range.contains(&cursor_byte) {
                        let line = buffer.byte_to_line(byte_range.range.start).ok()?;
                        Some((line, byte_range))
                    } else {
                        None
                    }
                })
                .sorted_by_key(|(line, byte_range)| {
                    (
                        // Prioritize same line
                        line.abs_diff(cursor_line),
                        // Then by nearest range start
                        byte_range.range.start.abs_diff(cursor_byte),
                    )
                })
                .next()
        } {
            Ok(Some(
                best_intersecting_match.to_selection(buffer, current_selection)?,
            ))
        }
        // If no intersecting match found, look in the reversed direction
        else {
            let result = match if_current_not_found {
                IfCurrentNotFound::LookForward => self.next(params.clone()),
                IfCurrentNotFound::LookBackward => self.previous(params.clone()),
            }?;
            if let Some(result) = result {
                Ok(Some(result))
            } else {
                // Look in another direction if matching selection is not found in the
                // preferred direction
                match if_current_not_found {
                    IfCurrentNotFound::LookForward => self.previous(params),
                    IfCurrentNotFound::LookBackward => self.next(params),
                }
            }
        }
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
        components::{
            editor::{Direction, IfCurrentNotFound, Movement},
            suggestive_editor::Info,
        },
        selection::{CharIndex, Selection},
        selection_mode::LineTrimmed,
    };

    use super::{ByteRange, SelectionMode, SelectionModeParams};
    use pretty_assertions::assert_eq;

    struct Dummy;
    impl SelectionMode for Dummy {
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
        test(
            Movement::Current(IfCurrentNotFound::LookForward),
            0..1,
            0..6,
        );
        test(
            Movement::Current(IfCurrentNotFound::LookForward),
            5..6,
            1..6,
        );
        test(
            Movement::Current(IfCurrentNotFound::LookForward),
            1..2,
            1..6,
        );
        test(
            Movement::Current(IfCurrentNotFound::LookForward),
            3..3,
            3..4,
        );
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
        };
        struct Dummy;
        impl SelectionMode for Dummy {
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
        run_test(
            Movement::Current(IfCurrentNotFound::LookForward),
            "Spongebob\n=======\nSquarepants",
        );
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
