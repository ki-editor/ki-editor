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
use std::{ops::Range, rc::Rc};
pub(crate) use syntax_node::SyntaxNode;
pub(crate) use syntax_token::SyntaxToken;
pub(crate) use token::Token;
pub(crate) use top_node::TopNode;
pub(crate) use word::Word;

use crate::{
    buffer::Buffer,
    char_index_range::{range_intersects, CharIndexRange},
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
        debug_assert!(
            range.end >= range.start,
            "range.end >= range.start, range = {range:?}"
        );
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

    fn expand(&self) -> Result<Option<ApplyMovementResult>, anyhow::Error> {
        let buffer = self.buffer;
        let selection = self.current_selection;
        let range = self.current_selection.extended_range();
        let range_start = self.current_selection.extended_range().start;
        let range_end = self.current_selection.extended_range().end;
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

pub(crate) fn get_current_selection_by_cursor_via_iter(
    buffer: &crate::buffer::Buffer,
    cursor_char_index: crate::selection::CharIndex,
    if_current_not_found: IfCurrentNotFound,
    vec: Rc<Vec<ByteRange>>,
) -> anyhow::Result<Option<ByteRange>> {
    debug_assert!(vec.iter().is_sorted());
    let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
    let mut previous_range: Option<&ByteRange> = None;
    let partition_point = vec.partition_point(|byte_range| byte_range.range().start < cursor_byte);
    let calibrated_partition_point = match if_current_not_found {
        IfCurrentNotFound::LookForward => partition_point,
        IfCurrentNotFound::LookBackward => {
            if partition_point == 0 {
                return Ok(None);
            } else {
                partition_point - 1
            }
        }
    };
    return Ok(vec.iter().nth(calibrated_partition_point).cloned());

    for range in vec.iter() {
        if range.range().contains(&cursor_byte) {
            return Ok(Some(range.clone()));
        } else if range.range.start > cursor_byte {
            match if_current_not_found {
                IfCurrentNotFound::LookForward => return Ok(Some(range.clone())),
                IfCurrentNotFound::LookBackward => return Ok(previous_range.cloned()),
            }
        } else {
            previous_range = Some(range)
        }
    }
    Ok(None)
}

/// This is so that any struct that implements PositionBasedSelectionMode
/// gets a free implementation of SelectionMode.
///
/// See https://stackoverflow.com/a/40945952/6587634
impl<T: PositionBasedSelectionMode> SelectionMode for PositionBased<T> {
    fn revealed_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.all_selections(params)
    }
    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        self.0
            .get_current_selection_by_cursor(buffer, cursor_char_index, if_current_not_found)
    }

    #[cfg(test)]
    fn all_selections_gathered_inversely<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let mut cursor_char_index = CharIndex(params.buffer.len_chars() - 1);
        let mut result = Vec::new();
        loop {
            if let Some(range) = self.get_current_selection_by_cursor(
                &params.buffer,
                cursor_char_index,
                IfCurrentNotFound::LookBackward,
            )? {
                if range.range.start == 0 || Some(&range) == result.first() {
                    result.insert(0, range);
                    break;
                } else {
                    cursor_char_index = params.buffer.byte_to_char(range.range.start - 1)?;
                    result.insert(0, range);
                }
            } else {
                break;
            }
        }
        return Ok(result);
    }

    fn to_index(
        &self,
        params: &SelectionModeParams,
        index: usize,
    ) -> anyhow::Result<Option<Selection>> {
        let current_selection = params.current_selection;
        let buffer = params.buffer;
        let mut cursor_char_index = CharIndex(0);
        let limit = CharIndex(params.buffer.len_chars());
        let mut current_index: usize = 0;
        while cursor_char_index < limit {
            if let Some(range) = self.get_current_selection_by_cursor(
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

    fn first(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.first(params)
    }

    fn last(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.last(params)
    }

    fn right(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.0.right(params)
    }

    fn left(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.0.left(params)
    }

    fn all_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.0.all_selections(params)
    }

    fn up(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.up_impl(params)
    }

    fn down(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.down_impl(params)
    }

    fn expand(&self, params: &SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.0.expand_impl(params)
    }

    fn jumps(
        &self,
        params: &SelectionModeParams,
        chars: Vec<char>,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<Jump>> {
        self.0.jumps_impl(params, chars, line_number_ranges)
    }

    fn selections_in_line_number_range(
        &self,
        params: &SelectionModeParams,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.0
            .selections_in_line_number_range_impl(params, line_number_ranges)
    }
}

pub trait SelectionMode {
    fn all_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>>;

    #[cfg(test)]
    fn all_selections_gathered_inversely<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>>;

    fn apply_movement(
        &self,
        params: &SelectionModeParams,
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
            Movement::Last => convert(self.last(&params)),
            Movement::Current(if_current_not_found) => {
                convert(self.current(params, if_current_not_found))
            }
            Movement::First => convert(self.first(&params)),
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

    fn expand(&self, params: &SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>>;

    fn up(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    fn down(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    fn selections_in_line_number_range(
        &self,
        params: &SelectionModeParams,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>>;

    fn revealed_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.all_selections(params)
    }

    fn jumps(
        &self,
        params: &SelectionModeParams,
        chars: Vec<char>,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<Jump>>;

    fn to_index(
        &self,
        params: &SelectionModeParams,
        index: usize,
    ) -> anyhow::Result<Option<Selection>>;

    fn delete_forward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.right(params)
    }

    fn delete_backward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.left(params)
    }

    fn first(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    fn last(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>>;

    fn current(
        &self,
        params: &SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let range = self.get_current_selection_by_cursor(
            &params.buffer,
            params.cursor_char_index(),
            if_current_not_found,
        )?;
        let range = if range.is_none() {
            self.get_current_selection_by_cursor(
                &params.buffer,
                params.cursor_char_index(),
                if_current_not_found.inverse(),
            )?
        } else {
            range
        };
        range
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
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>>;

    fn left(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>>;

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

        let actual_forward = self
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

        assert_eq!(expected, actual_forward);

        let actual_backward = self
            .all_selections_gathered_inversely(SelectionModeParams {
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

        assert_eq!(expected, actual_backward, "backward assertion");
    }
}

pub trait PositionBasedSelectionMode {
    fn jumps_impl(
        &self,
        params: &SelectionModeParams,
        chars: Vec<char>,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<Jump>> {
        let ranges = self.selections_in_line_number_range_impl(&params, line_number_ranges)?;
        let jumps = ranges
            .into_iter()
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

    fn selections_in_line_number_range_impl(
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
                match self.get_current_selection_by_cursor(
                    &params.buffer,
                    cursor_char_index,
                    IfCurrentNotFound::LookBackward,
                )? {
                    Some(range)
                        if byte_ranges
                            .iter()
                            .any(|byte_range| byte_range.contains(&range.range.start)) =>
                    {
                        let range_start = range.range.start;
                        if result.iter().any(|existing_range| existing_range == &range) {
                            break result;
                        } else {
                            result.push(range);
                            cursor_char_index = params.buffer.byte_to_char(range_start)? - 1;
                        }
                    }
                    _ => break result,
                }
            }
        };

        let after_cursor_selections = {
            let mut result = Vec::new();
            let mut cursor_char_index = params.current_selection.range().end;
            let last_range = self.last(params)?.and_then(|selection| {
                params
                    .buffer
                    .char_index_range_to_byte_range(selection.range())
                    .ok()
            });
            loop {
                match self.get_current_selection_by_cursor(
                    &params.buffer,
                    cursor_char_index,
                    IfCurrentNotFound::LookForward,
                )? {
                    Some(range)
                        if byte_ranges
                            .iter()
                            .any(|byte_range| byte_range.contains(&range.range.start)) =>
                    {
                        let range_end = range.range.end;
                        if result.iter().any(|existing_range| existing_range == &range) {
                            break result;
                        } else {
                            result.push(range);
                            cursor_char_index = params.buffer.byte_to_char(range_end)?;
                        }
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
    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>>;

    fn first(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_current_selection_by_cursor(
            &params.buffer,
            CharIndex(0),
            IfCurrentNotFound::LookForward,
        )?
        .map(|byte_range| byte_range.to_selection(params.buffer, params.current_selection))
        .transpose()
    }

    fn last(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_current_selection_by_cursor(
            &params.buffer,
            CharIndex(params.buffer.len_chars()) - 1,
            IfCurrentNotFound::LookBackward,
        )?
        .map(|byte_range| byte_range.to_selection(params.buffer, params.current_selection))
        .transpose()
    }

    fn right(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.get_current_selection_by_cursor(
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
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.get_current_selection_by_cursor(
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

    fn delete_forward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.right(params)
    }

    fn delete_backward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.left(params)
    }

    fn revealed_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.all_selections(params)
    }

    fn all_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let mut cursor_char_index = CharIndex(0);
        let mut result = Vec::new();
        while cursor_char_index < CharIndex(params.buffer.len_chars()) {
            if let Some(range) = self.get_current_selection_by_cursor(
                &params.buffer,
                cursor_char_index,
                IfCurrentNotFound::LookForward,
            )? {
                cursor_char_index = params.buffer.byte_to_char(range.range.end)?;

                if Some(&range) == result.last() {
                    result.push(range);
                    break;
                } else {
                    result.push(range);
                }
            } else {
                break;
            }
        }
        return Ok(result);
    }

    fn expand_impl(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        params.expand()
    }

    fn up_impl(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.vertical_movement(params, true)
    }

    fn down_impl(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.vertical_movement(params, false)
    }

    fn vertical_movement(
        &self,
        params: &SelectionModeParams,
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

        while let Some(result) =
            self.get_current_selection_by_cursor(&params.buffer, new_cursor_char_index, first_look)?
        {
            if buffer.byte_to_line(result.range.start)? == new_position.line {
                return Ok(Some(
                    (*current_selection)
                        .clone()
                        .set_range(buffer.byte_range_to_char_index_range(&result.range)?)
                        .set_info(result.info),
                ));
            } else if let Some(result) = self.get_current_selection_by_cursor(
                &params.buffer,
                new_cursor_char_index,
                second_look,
            )? {
                if buffer.byte_to_line(result.range.start)? == new_position.line {
                    return Ok(Some(
                        (*current_selection)
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
}
pub(crate) struct PositionBased<T: PositionBasedSelectionMode>(pub(crate) T);
pub(crate) struct VectorBased<T: VectorBasedSelectionMode>(pub(crate) T);

impl<T: VectorBasedSelectionMode> SelectionMode for VectorBased<T> {
    fn all_selections<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        Ok(self
            .0
            .get_byte_ranges(params.buffer)?
            .iter()
            .cloned()
            .collect_vec())
    }

    #[cfg(test)]
    fn all_selections_gathered_inversely<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.all_selections(params)
    }

    fn selections_in_line_number_range(
        &self,
        params: &SelectionModeParams,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let line_byte_ranges = line_number_ranges
            .into_iter()
            .flat_map(|lines| lines.flat_map(|line| params.buffer.line_to_byte_range(line).ok()))
            .collect_vec();
        Ok(self
            .0
            .get_byte_ranges(params.buffer)?
            .iter()
            .filter(|range| {
                line_byte_ranges
                    .iter()
                    .any(|line_byte_range| range_intersects(line_byte_range.range(), range.range()))
            })
            .cloned()
            .collect_vec())
    }

    fn to_index(
        &self,
        params: &SelectionModeParams,
        index: usize,
    ) -> anyhow::Result<Option<Selection>> {
        self.0
            .get_byte_ranges(params.buffer)?
            .get(index)
            .map(|range| {
                Ok(params.current_selection.clone().set_range(
                    params
                        .buffer
                        .byte_range_to_char_index_range(range.range())?,
                ))
            })
            .transpose()
    }

    fn first(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.to_index(params, 0)
    }

    fn last(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.to_index(
            params,
            self.0
                .get_byte_ranges(params.buffer)?
                .len()
                .saturating_sub(1),
        )
    }

    fn right(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let byte_range = params
            .buffer
            .char_index_range_to_byte_range(params.current_selection.range())?;
        let ranges = self.0.get_byte_ranges(params.buffer)?;
        match ranges.binary_search_by(|range| range.range().clone().cmp(byte_range.clone())) {
            Ok(index) if index < ranges.len().saturating_sub(1) => ranges
                .get(index + 1)
                .map(|byte_range| {
                    Ok(params.current_selection.clone().set_range(
                        params
                            .buffer
                            .byte_range_to_char_index_range(byte_range.range())?,
                    ))
                })
                .transpose(),
            _ => self.current(&params, IfCurrentNotFound::LookForward),
        }
    }

    fn left(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let byte_ranges = self.0.get_byte_ranges(params.buffer)?;
        let byte_range = params
            .buffer
            .char_index_range_to_byte_range(params.current_selection.range())?;
        match byte_ranges.binary_search_by(|range| range.range().clone().cmp(byte_range.clone())) {
            Ok(index) if index > 0 => byte_ranges
                .get(index - 1)
                .map(|byte_range| {
                    Ok(params.current_selection.clone().set_range(
                        params
                            .buffer
                            .byte_range_to_char_index_range(byte_range.range())?,
                    ))
                })
                .transpose(),
            _ => self.current(&params, IfCurrentNotFound::LookBackward),
        }
    }

    fn up(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.up_impl(params)
    }

    fn down(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.down_impl(params)
    }

    fn expand(&self, params: &SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        params.expand()
    }

    fn jumps(
        &self,
        params: &SelectionModeParams,
        chars: Vec<char>,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<Jump>> {
        todo!()
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        self.0
            .get_current_selection_by_cursor(buffer, cursor_char_index, if_current_not_found)
    }
}

pub trait VectorBasedSelectionMode {
    fn get_byte_ranges(&self, buffer: &Buffer) -> anyhow::Result<Rc<Vec<ByteRange>>>;

    fn up_impl(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.vertical_movement(params, true)
    }

    fn down_impl(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.vertical_movement(params, false)
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        let vec = self.get_byte_ranges(buffer)?;
        debug_assert!(vec.iter().is_sorted());
        let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
        let mut previous_range: Option<&ByteRange> = None;
        let partition_point =
            vec.partition_point(|byte_range| byte_range.range().start < cursor_byte);
        let calibrated_partition_point = match if_current_not_found {
            IfCurrentNotFound::LookForward => partition_point,
            IfCurrentNotFound::LookBackward => {
                if partition_point == 0 {
                    return Ok(None);
                } else {
                    partition_point - 1
                }
            }
        };
        return Ok(vec.iter().nth(calibrated_partition_point).cloned());

        for range in vec.iter() {
            if range.range().contains(&cursor_byte) {
                return Ok(Some(range.clone()));
            } else if range.range.start > cursor_byte {
                match if_current_not_found {
                    IfCurrentNotFound::LookForward => return Ok(Some(range.clone())),
                    IfCurrentNotFound::LookBackward => return Ok(previous_range.cloned()),
                }
            } else {
                previous_range = Some(range)
            }
        }
        Ok(None)
    }

    fn vertical_movement(
        &self,
        params: &SelectionModeParams,
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

        while let Some(result) =
            self.get_current_selection_by_cursor(&params.buffer, new_cursor_char_index, first_look)?
        {
            if buffer.byte_to_line(result.range.start)? == new_position.line {
                return Ok(Some(
                    (*current_selection)
                        .clone()
                        .set_range(buffer.byte_range_to_char_index_range(&result.range)?)
                        .set_info(result.info),
                ));
            } else if let Some(result) = self.get_current_selection_by_cursor(
                &params.buffer,
                new_cursor_char_index,
                second_look,
            )? {
                if buffer.byte_to_line(result.range.start)? == new_position.line {
                    return Ok(Some(
                        (*current_selection)
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
        selection_mode::{PositionBased, SelectionMode},
    };

    use super::{ByteRange, PositionBasedSelectionMode, SelectionModeParams};
    use pretty_assertions::assert_eq;

    struct Dummy;
    impl PositionBasedSelectionMode for Dummy {
        fn get_current_selection_by_cursor(
            &self,
            buffer: &crate::buffer::Buffer,
            cursor_char_index: crate::selection::CharIndex,
            _: crate::components::editor::IfCurrentNotFound,
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
        let actual = PositionBased(Dummy)
            .apply_movement(&params, movement)
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
        impl PositionBasedSelectionMode for Dummy {
            fn get_current_selection_by_cursor(
                &self,
                buffer: &crate::buffer::Buffer,
                cursor_char_index: crate::selection::CharIndex,
                if_current_not_found: crate::components::editor::IfCurrentNotFound,
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
            let actual = PositionBased(Dummy)
                .apply_movement(&params, movement)
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
        let actual = PositionBased(Dummy)
            .apply_movement(&params, Movement::Right)
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
