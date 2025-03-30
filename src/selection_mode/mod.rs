pub(crate) mod ast_grep;
pub(crate) mod character;
pub(crate) mod custom;
pub(crate) mod diagnostic;
pub(crate) mod git_hunk;
pub(crate) mod mark;
pub(crate) mod naming_convention_agnostic;
pub(crate) mod syntax_token;

pub(crate) mod top_node;

pub(crate) mod line_full;
pub(crate) mod line_trimmed;
pub(crate) mod local_quickfix;
pub(crate) mod regex;
pub(crate) mod syntax_node;
pub(crate) mod token;
pub(crate) mod word;
pub(crate) use self::regex::Regex;
pub(crate) use ast_grep::AstGrep;
pub(crate) use character::Character;
pub(crate) use custom::Custom;
pub(crate) use diagnostic::Diagnostic;
pub(crate) use git_hunk::GitHunk;
use itertools::Itertools;
pub(crate) use line_full::LineFull;
pub(crate) use line_trimmed::LineTrimmed;
pub(crate) use local_quickfix::LocalQuickfix;
pub(crate) use mark::Mark;
pub(crate) use naming_convention_agnostic::NamingConventionAgnostic;
use position_pair::ParsedChar;
use ropey::Rope;
use std::ops::Range;
pub(crate) use syntax_node::SyntaxNode;
pub(crate) use syntax_token::SyntaxToken;
pub(crate) use token::Token;
pub(crate) use top_node::TopNode;
pub(crate) use word::Word;

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        editor::{Direction, IfCurrentNotFound, Jump, Movement, SurroundKind},
        suggestive_editor::Info,
    },
    selection::{CharIndex, Selection},
    surround::EnclosureKind,
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

    pub(crate) fn info(&self) -> Option<Info> {
        self.info.clone()
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
impl SelectionModeParams<'_> {
    fn cursor_char_index(&self) -> CharIndex {
        self.current_selection.to_char_index(self.cursor_direction)
    }

    fn cursor_byte(&self) -> anyhow::Result<usize> {
        self.buffer.char_to_byte(self.cursor_char_index())
    }
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
    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
    ) -> anyhow::Result<Option<ByteRange>>;

    fn all_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let mut cursor_char_index = CharIndex(0);
        let mut result = Vec::new();
        while cursor_char_index < CharIndex(params.buffer.len_chars()) {
            if let Some(range) = self.get_current_selection_by_cursor_with_look(
                &params.buffer,
                cursor_char_index,
                IfCurrentNotFound::LookForward,
            )? {
                cursor_char_index = params.buffer.byte_to_char(range.range.end)?;

                if Some(&range) == result.last() {
                    break;
                }

                result.push(range);
            } else {
                break;
            }
        }
        return Ok(result);
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
            Movement::Right => convert(self.right(params)),

            Movement::Left => convert(self.left(params)),
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
            Movement::Expand => self.expand(params),
            Movement::DeleteBackward => convert(self.delete_backward(params)),
            Movement::DeleteForward => convert(self.delete_forward(params)),
        }
    }

    fn expand(&self, params: SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        let buffer = params.buffer;
        let selection = params.current_selection;
        let range = params.current_selection.extended_range();
        let range_start = params.current_selection.extended_range().start;
        let range_end = params.current_selection.extended_range().end;
        use position_pair::Position::*;
        use EnclosureKind::*;
        use SurroundKind::*;
        let enclosure_kinds = {
            [
                Parentheses,
                CurlyBraces,
                SquareBrackets,
                DoubleQuotes,
                SingleQuotes,
                Backticks,
            ]
        };
        let positioned_chars =
            position_pair::create_position_pairs(&buffer.content().chars().collect_vec());
        let (enclosure_kind, surround_kind, open_index, close_index) = match (
            positioned_chars.get(range_start.0),
            positioned_chars.get(range_end.0.saturating_sub(1)),
        ) {
            (Some(ParsedChar::Enclosure(Open, open_kind)), end) => match end {
                Some(ParsedChar::Enclosure(Close, close_kind)) if open_kind == close_kind => {
                    (None, None, None, None)
                }
                _ => {
                    match (
                        positioned_chars.get(range_start.0.saturating_sub(1)),
                        positioned_chars.get(range_end.0),
                    ) {
                        (
                            Some(ParsedChar::Enclosure(Open, open_kind)),
                            Some(ParsedChar::Enclosure(Close, close_kind)),
                        ) if open_kind == close_kind => (
                            Some(open_kind),
                            Some(Around),
                            Some(range_start.0.saturating_sub(1)),
                            Some(range_end.0),
                        ),
                        _ => (Some(open_kind), Some(Around), Some(range_start.0), None),
                    }
                }
            },
            (Some(ParsedChar::Enclosure(Close, kind)), _) => {
                (Some(kind), Some(Around), None, Some(range_start.0))
            }
            _ => (None, None, None, None),
        };
        let (close_index, enclosure_kind) = match (close_index, enclosure_kind) {
            (Some(close_index), Some(enclosure_kind)) => (close_index, *enclosure_kind),
            _ => {
                let mut after = positioned_chars
                    .iter()
                    .enumerate()
                    .skip(range_start.0 + if open_index.is_some() { 1 } else { 0 });
                // This stack is for handling nested enclosures
                let mut open_symbols_stack = Vec::new();
                let (close_index, enclosure_kind) = {
                    let Some((close_index, enclosure_kind)) =
                        after.find_map(|(index, positioned_char)| {
                            if let Some(kind) = enclosure_kinds
                                .iter()
                                .find(|kind| positioned_char.is_opening_of(kind))
                                .cloned()
                            {
                                open_symbols_stack.push(kind);
                            } else if let Some(kind) = enclosure_kind
                                .and_then(|kind| {
                                    if positioned_char.is_closing_of(kind) {
                                        Some(kind)
                                    } else {
                                        None
                                    }
                                })
                                .or_else(|| {
                                    enclosure_kinds
                                        .iter()
                                        .find(|kind| positioned_char.is_closing_of(kind))
                                })
                            {
                                if open_symbols_stack.last() == Some(kind) {
                                    open_symbols_stack.pop();
                                } else {
                                    return Some((index, *kind));
                                }
                            }
                            None
                        })
                    else {
                        return Ok(None);
                    };
                    (close_index, enclosure_kind)
                };
                (close_index, enclosure_kind)
            }
        };
        let open_index = if let Some(open_index) = open_index {
            open_index
        } else {
            let slice_range = 0..range_start.0;
            let before = &positioned_chars[slice_range];
            // This stack is for handling nested enclosures
            let mut close_symbols_stack = Vec::new();
            let Some(open_index) = before
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, positioned_char)| {
                    if positioned_char.is_closing_of(&enclosure_kind) {
                        close_symbols_stack.push(enclosure_kind);
                    } else if positioned_char.is_opening_of(&enclosure_kind) {
                        if close_symbols_stack.last() == Some(&enclosure_kind) {
                            close_symbols_stack.pop();
                        } else {
                            return Some((index, enclosure_kind));
                        }
                    }
                    None
                })
                .map(|(index, _)| index)
            else {
                return Ok(None);
            };
            open_index
        };
        let surround_kind = if let Some(surround_kind) = surround_kind {
            surround_kind
        } else if open_index + 1 == range.start.0 && close_index == range.end.0 {
            Around
        } else {
            Inside
        };
        let offset = match surround_kind {
            Inside => 1,
            Around => 0,
        };
        let range = (CharIndex(open_index + offset)..CharIndex(close_index + 1 - offset)).into();
        Ok(Some(ApplyMovementResult::from_selection(
            selection.clone().set_range(range),
        )))
    }

    fn up(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.vertical_movement(params, true)
    }

    fn down(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.vertical_movement(params, false)
    }

    fn vertical_movement(
        &self,
        params: SelectionModeParams,
        is_up: bool,
    ) -> anyhow::Result<Option<Selection>> {
        let cursor_char_index = params.cursor_char_index();
        let SelectionModeParams {
            buffer,
            current_selection,
            ..
        } = params;
        let current_position = buffer.char_to_position(cursor_char_index)?;

        // Early return check
        if (is_up && current_position.line == 0)
            || (!is_up && current_position.line == buffer.len_lines().saturating_sub(1))
        {
            return Ok(None);
        }

        // Calculate the new line
        let new_line = if is_up {
            current_position.line - 1
        } else {
            current_position.line + 1
        };

        let mut new_position = current_position.set_line(new_line);
        let mut new_cursor_char_index = buffer.position_to_char(new_position)?;

        // Define which look direction to try first and second based on movement direction
        let (first_look, second_look) = if is_up {
            (
                IfCurrentNotFound::LookBackward,
                IfCurrentNotFound::LookForward,
            )
        } else {
            (
                IfCurrentNotFound::LookForward,
                IfCurrentNotFound::LookBackward,
            )
        };

        while let Some(result) = self.get_current_selection_by_cursor_with_look(
            &params.buffer,
            cursor_char_index,
            first_look,
        )? {
            if buffer.byte_to_line(result.range.start)? == new_position.line {
                return Ok(Some(
                    current_selection
                        .clone()
                        .set_range(buffer.byte_range_to_char_index_range(&result.range)?)
                        .set_info(result.info),
                ));
            } else if let Some(result) = self.get_current_selection_by_cursor_with_look(
                &params.buffer,
                cursor_char_index,
                second_look,
            )? {
                if buffer.byte_to_line(result.range.start)? == new_position.line {
                    return Ok(Some(
                        current_selection
                            .clone()
                            .set_range(buffer.byte_range_to_char_index_range(&result.range)?)
                            .set_info(result.info),
                    ));
                }
            }

            // Move to next line
            new_position.line = if is_up {
                new_position.line.saturating_sub(1)
            } else {
                new_position.line + 1
            };
            new_cursor_char_index = buffer.position_to_char(new_position)?;
        }

        Ok(None)
    }

    fn selections_in_line_number_range(
        &self,
        params: &SelectionModeParams,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let byte_ranges: Vec<_> = line_number_ranges
            .clone()
            .into_iter()
            .filter_map(|range| {
                Some(
                    params.buffer.line_to_byte(range.start).ok()?
                        ..params.buffer.line_to_byte(range.end).ok()?,
                )
            })
            .collect();

        let before_cursor_selections = {
            let mut result = Vec::new();
            let mut cursor_char_index = params.current_selection.range().start - 1;
            loop {
                match self.get_current_selection_by_cursor_with_look(
                    &params.buffer,
                    cursor_char_index,
                    IfCurrentNotFound::LookBackward,
                )? {
                    Some(range)
                        if byte_ranges
                            .iter()
                            .any(|byte_range| byte_range.contains(&range.range.start)) =>
                    {
                        cursor_char_index = params.buffer.byte_to_char(range.range.start)? - 1;
                        result.push(range);
                    }
                    _ => break result,
                }
            }
        };

        let after_cursor_selections = {
            let mut result = Vec::new();
            let mut cursor_char_index = params.current_selection.range().end;
            loop {
                match self.get_current_selection_by_cursor_with_look(
                    &params.buffer,
                    cursor_char_index,
                    IfCurrentNotFound::LookForward,
                )? {
                    Some(range)
                        if byte_ranges
                            .iter()
                            .any(|byte_range| byte_range.contains(&range.range.start)) =>
                    {
                        cursor_char_index = params.buffer.byte_to_char(range.range.start)? + 1;
                        result.push(range);
                    }
                    _ => break result,
                }
            }
        };

        let result = before_cursor_selections
            .into_iter()
            .chain(after_cursor_selections)
            .collect_vec();

        Ok(result)
    }

    fn revealed_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.all_selections(params)
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
            .chunk_by(|jump| jump.character)
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

    fn to_index(
        &self,
        params: SelectionModeParams,
        index: usize,
    ) -> anyhow::Result<Option<Selection>> {
        let current_selection = params.current_selection;
        let buffer = params.buffer;
        let mut cursor_char_index = CharIndex(0);
        let limit = CharIndex(params.buffer.len_chars());
        let mut current_index: usize = 0;
        while cursor_char_index < limit {
            if let Some(range) = self.get_current_selection_by_cursor_with_look(
                &params.buffer,
                cursor_char_index,
                IfCurrentNotFound::LookForward,
            )? {
                if current_index == index {
                    return Ok(Some(range.to_selection(buffer, current_selection)?));
                } else {
                    current_index += 1;
                    cursor_char_index = buffer.byte_to_char(range.range.end)?
                }
            } else {
                return Ok(None);
            }
        }
        return Ok(None);
    }

    fn delete_forward(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.right(params)
    }

    fn delete_backward(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.left(params)
    }

    fn first(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_current_selection_by_cursor_with_look(
            &params.buffer,
            CharIndex(0),
            IfCurrentNotFound::LookForward,
        )?
        .map(|byte_range| byte_range.to_selection(params.buffer, params.current_selection))
        .transpose()
    }

    fn last(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_current_selection_by_cursor_with_look(
            &params.buffer,
            CharIndex(params.buffer.len_chars()) - 1,
            IfCurrentNotFound::LookBackward,
        )?
        .map(|byte_range| byte_range.to_selection(params.buffer, params.current_selection))
        .transpose()
    }

    fn get_current_selection_by_cursor_with_look(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        // TODO: get_current_selection_by_cursor should not take `if_current_not_found`
        // if get_current_selection_by_cursor is None, then recursively call `self.left` or `self.right` depending on `if_current_not_found`
        // until found
        let mut cursor_char_index = cursor_char_index;
        let limit = CharIndex(0)..CharIndex(buffer.len_chars());
        while limit.contains(&cursor_char_index) {
            if let Some(range) = self.get_current_selection_by_cursor(buffer, cursor_char_index)? {
                return Ok(Some(range));
            } else {
                match if_current_not_found {
                    IfCurrentNotFound::LookForward => cursor_char_index = cursor_char_index + 1,
                    IfCurrentNotFound::LookBackward => cursor_char_index = cursor_char_index - 1,
                }
            }
        }

        Ok(None)
    }
    fn current(
        &self,
        params: SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.get_current_selection_by_cursor_with_look(
            &params.buffer,
            params.cursor_char_index(),
            if_current_not_found,
        )?
        .map(|range| {
            params
                .current_selection
                .clone()
                .update_with_byte_range(params.buffer, range)
        })
        .transpose()
    }
    fn right(
        &self,
        params: SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.get_current_selection_by_cursor_with_look(
            params.buffer,
            params.current_selection.range().end,
            IfCurrentNotFound::LookForward,
        )?
        .map(|range| {
            params
                .current_selection
                .clone()
                .update_with_byte_range(params.buffer, range)
        })
        .transpose()
    }
    fn left(
        &self,
        params: SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.get_current_selection_by_cursor_with_look(
            &params.buffer,
            params.current_selection.range().start - 1,
            IfCurrentNotFound::LookBackward,
        )?
        .map(|range| {
            params
                .current_selection
                .clone()
                .update_with_byte_range(params.buffer, range)
        })
        .transpose()
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
            .all_selections(SelectionModeParams {
                buffer,
                current_selection: &current_selection,
                cursor_direction: &Direction::default(),
            })
            .unwrap()
            .into_iter()
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
    };

    use super::{ByteRange, SelectionMode, SelectionModeParams};
    use pretty_assertions::assert_eq;

    struct Dummy;
    impl SelectionMode for Dummy {
        fn get_current_selection_by_cursor(
            &self,
            buffer: &crate::buffer::Buffer,
            cursor_char_index: crate::selection::CharIndex,
        ) -> anyhow::Result<Option<super::ByteRange>> {
            todo!()
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

        assert_eq!(
            expected, actual,
            "Input range = {current_selection_byte_range:?}, expected = {expected:?}, actual = {actual:?}"
        );
    }

    #[test]
    fn previous() {
        test(Movement::Left, 1..6, 0..6);
        test(Movement::Left, 2..5, 1..6);
        test(Movement::Left, 3..5, 3..4);

        test(Movement::Left, 3..4, 2..5);
    }

    #[test]
    fn next() {
        test(Movement::Right, 0..6, 1..6);
        test(Movement::Right, 1..6, 2..5);
        test(Movement::Right, 2..5, 3..4);
        test(Movement::Right, 3..4, 3..5);
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
            fn get_current_selection_by_cursor(
                &self,
                buffer: &crate::buffer::Buffer,
                cursor_char_index: crate::selection::CharIndex,
            ) -> anyhow::Result<Option<super::ByteRange>> {
                let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
                Ok([
                    ByteRange::with_info(
                        1..2,
                        Info::new("Title".to_string(), "Spongebob".to_string()),
                    ),
                    ByteRange::with_info(
                        1..2,
                        Info::new("Title".to_string(), "Squarepants".to_string()),
                    ),
                ]
                .into_iter()
                .find(|range| range.range.contains(&cursor_byte)))
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
            .apply_movement(params, Movement::Right)
            .unwrap()
            .unwrap()
            .selection
            .range();
        let expected: CharIndexRange = (CharIndex(3)..CharIndex(4)).into();

        assert_eq!(expected, actual);
    }
}

mod position_pair {
    use crate::surround::EnclosureKind;

    #[derive(Debug, PartialEq)]
    pub(crate) enum Position {
        Open,
        Close,
        Escaped,
    }

    #[derive(Debug, PartialEq)]
    pub(crate) enum ParsedChar {
        Enclosure(Position, EnclosureKind),
        Other(char),
    }

    impl ParsedChar {
        pub(crate) fn is_closing_of(
            &self,
            enclosure_kind: &crate::surround::EnclosureKind,
        ) -> bool {
            match self {
                ParsedChar::Enclosure(Position::Close, kind) => kind == enclosure_kind,
                _ => false,
            }
        }

        pub(crate) fn is_opening_of(
            &self,
            enclosure_kind: &crate::surround::EnclosureKind,
        ) -> bool {
            match self {
                ParsedChar::Enclosure(Position::Open, kind) => kind == enclosure_kind,
                _ => false,
            }
        }
    }

    pub(crate) fn create_position_pairs(chars: &[char]) -> Vec<ParsedChar> {
        use EnclosureKind::*;
        use Position::*;
        let mut parsed_chars = Vec::new();
        let mut char_counts: std::collections::HashMap<char, usize> =
            std::collections::HashMap::new();

        for i in 0..chars.len() {
            let c = chars[i];
            let count = char_counts.entry(c).or_insert(0);
            let position = if (c == '\'' || c == '"' || c == '`') && i > 0 && chars[i - 1] == '\\' {
                Position::Escaped
            } else {
                *count += 1;
                if *count % 2 == 0 {
                    Position::Close
                } else {
                    Position::Open
                }
            };
            let parsed_char = match c {
                '\'' => ParsedChar::Enclosure(position, SingleQuotes),
                '"' => ParsedChar::Enclosure(position, DoubleQuotes),
                '`' => ParsedChar::Enclosure(position, Backticks),
                '{' => ParsedChar::Enclosure(Open, CurlyBraces),
                '}' => ParsedChar::Enclosure(Close, CurlyBraces),
                '(' => ParsedChar::Enclosure(Open, Parentheses),
                ')' => ParsedChar::Enclosure(Close, Parentheses),
                '[' => ParsedChar::Enclosure(Open, SquareBrackets),
                ']' => ParsedChar::Enclosure(Close, SquareBrackets),
                other => ParsedChar::Other(other),
            };
            parsed_chars.push(parsed_char);
        }

        parsed_chars
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use itertools::Itertools;
        use EnclosureKind::*;
        use ParsedChar::*;
        use Position::*;

        #[test]
        fn test_position_pairs() {
            let input = vec!['a', 'b', 'a', 'c', 'b', 'a'];
            let expected = vec![
                Other('a'),
                Other('b'),
                Other('a'),
                Other('c'),
                Other('b'),
                Other('a'),
            ];
            assert_eq!(create_position_pairs(&input), expected);
        }

        #[test]
        fn test_single_chars() {
            let input = vec!['x', 'y', 'z'];
            let expected = vec![Other('x'), Other('y'), Other('z')];
            assert_eq!(create_position_pairs(&input), expected);
        }

        #[test]
        fn test_escaped_single_quote() {
            let input = vec!['\'', '\\', '\'', '\''];
            let expected = vec![
                Enclosure(Open, SingleQuotes),
                Other('\\'),
                Enclosure(Escaped, SingleQuotes),
                Enclosure(Close, SingleQuotes),
            ];
            assert_eq!(create_position_pairs(&input), expected);
        }

        #[test]
        fn test_escaped_double_quote() {
            let input = vec!['"', '\\', '"', '"'];
            let expected = vec![
                Enclosure(Open, DoubleQuotes),
                Other('\\'),
                Enclosure(Escaped, DoubleQuotes),
                Enclosure(Close, DoubleQuotes),
            ];
            assert_eq!(create_position_pairs(&input), expected);
        }

        #[test]
        fn test_escaped_backtick() {
            let input = vec!['`', '\\', '`', '`'];
            let expected = vec![
                Enclosure(Open, Backticks),
                Other('\\'),
                Enclosure(Escaped, Backticks),
                Enclosure(Close, Backticks),
            ];
            assert_eq!(create_position_pairs(&input), expected);
        }

        #[test]
        fn test_all_enclosure_types() {
            let input = r#"{(['"`xyz\'\"\`])}'"`"#.chars().collect_vec();
            let expected = vec![
                Enclosure(Open, CurlyBraces),
                Enclosure(Open, Parentheses),
                Enclosure(Open, SquareBrackets),
                Enclosure(Open, SingleQuotes),
                Enclosure(Open, DoubleQuotes),
                Enclosure(Open, Backticks),
                Other('x'),
                Other('y'),
                Other('z'),
                Other('\\'),
                Enclosure(Escaped, SingleQuotes),
                Other('\\'),
                Enclosure(Escaped, DoubleQuotes),
                Other('\\'),
                Enclosure(Escaped, Backticks),
                Enclosure(Close, SquareBrackets),
                Enclosure(Close, Parentheses),
                Enclosure(Close, CurlyBraces),
                Enclosure(Close, SingleQuotes),
                Enclosure(Close, DoubleQuotes),
                Enclosure(Close, Backticks),
            ];
            assert_eq!(create_position_pairs(&input), expected);
        }
    }
}
