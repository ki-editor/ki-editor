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
pub(crate) mod subword;
pub(crate) mod syntax_node;
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
use std::ops::Range;
pub(crate) use subword::Subword;
pub(crate) use syntax_node::SyntaxNode;
pub(crate) use top_node::TopNode;
pub(crate) use word::Word;

use crate::{
    buffer::Buffer,
    char_index_range::{range_intersects, CharIndexRange},
    components::{
        editor::{Direction, IfCurrentNotFound, Jump, MovementApplicandum, SurroundKind},
        suggestive_editor::Info,
    },
    position::Position,
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

pub(crate) struct SelectionModeParams<'a> {
    pub(crate) buffer: &'a Buffer,
    pub(crate) current_selection: &'a Selection,
    pub(crate) cursor_direction: &'a Direction,
}
impl SelectionModeParams<'_> {
    fn cursor_char_index(&self) -> CharIndex {
        self.current_selection.to_char_index(self.cursor_direction)
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
    pub(crate) sticky_column_index: Option<usize>,
}

impl ApplyMovementResult {
    pub(crate) fn from_selection(selection: Selection) -> Self {
        Self {
            selection,
            mode: None,
            sticky_column_index: None,
        }
    }
}

/// This is so that any struct that implements PositionBasedSelectionMode
/// gets a free implementation of SelectionMode.
///
/// See https://stackoverflow.com/a/40945952/6587634
impl<T: PositionBasedSelectionMode> SelectionModeTrait for PositionBased<T> {
    fn revealed_selections<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.0.revealed_selections(params)
    }

    #[cfg(test)]
    fn all_selections_gathered_inversely<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.0.all_selections_gathered_inversely(params)
    }

    fn to_index(
        &self,
        params: &SelectionModeParams,
        index: usize,
    ) -> anyhow::Result<Option<Selection>> {
        self.0.to_index(params, index)
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
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.0.all_selections(params)
    }

    fn up(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.0.up(params, sticky_column_index)
    }

    fn down(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.0.down(params, sticky_column_index)
    }

    fn expand(&self, params: &SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.0.expand(params)
    }

    fn selections_in_line_number_ranges(
        &self,
        params: &SelectionModeParams,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.0
            .selections_in_line_number_ranges(params, line_number_ranges)
    }

    fn delete_forward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.delete_forward(params)
    }

    fn delete_backward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.delete_backward(params)
    }

    fn current(
        &self,
        params: &SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.0.current(params, if_current_not_found)
    }

    fn process_paste_gap(
        &self,
        params: &SelectionModeParams,
        prev_gap: Option<String>,
        next_gap: Option<String>,
        direction: &Direction,
    ) -> String {
        self.0
            .process_paste_gap(params, prev_gap, next_gap, direction)
    }

    fn next(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.next(params)
    }

    fn previous(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.previous(params)
    }

    fn alpha(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.alpha(params)
    }

    fn omega(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.omega(params)
    }
}

pub trait SelectionModeTrait {
    fn all_selections<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.selections_in_line_number_ranges(
            params,
            Some(0..params.buffer.len_lines()).into_iter().collect(),
        )
    }

    #[cfg(test)]
    fn all_selections_gathered_inversely<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>>;

    fn apply_movement(
        &self,
        params: &SelectionModeParams,
        movement: MovementApplicandum,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        fn convert(
            result: anyhow::Result<Option<Selection>>,
        ) -> anyhow::Result<Option<ApplyMovementResult>> {
            Ok(result?.map(|result| result.into()))
        }
        match movement {
            MovementApplicandum::Right => convert(self.right(params)),
            MovementApplicandum::Left => convert(self.left(params)),
            MovementApplicandum::Last => convert(self.last(params)),
            MovementApplicandum::Current(if_current_not_found) => {
                convert(self.current(params, if_current_not_found))
            }
            MovementApplicandum::First => convert(self.first(params)),
            MovementApplicandum::Index(index) => convert(self.to_index(params, index)),
            MovementApplicandum::Jump(range) => Ok(Some(ApplyMovementResult::from_selection(
                params.current_selection.clone().set_range(range),
            ))),
            MovementApplicandum::Up {
                sticky_column_index,
            } => self.up(params, sticky_column_index),
            MovementApplicandum::Down {
                sticky_column_index,
            } => self.down(params, sticky_column_index),
            MovementApplicandum::Expand => self.expand(params),
            MovementApplicandum::DeleteBackward => convert(self.delete_backward(params)),
            MovementApplicandum::DeleteForward => convert(self.delete_forward(params)),
            MovementApplicandum::Next => convert(self.next(params)),
            MovementApplicandum::Previous => convert(self.previous(params)),
        }
    }

    fn expand(&self, params: &SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>>;

    fn up(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>>;

    fn down(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>>;

    fn next(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    fn previous(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    fn selections_in_line_number_ranges(
        &self,
        params: &SelectionModeParams,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>>;

    fn revealed_selections<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.all_selections(params)
    }

    fn jumps(
        &self,
        params: &SelectionModeParams,
        chars: Vec<char>,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<Jump>> {
        let ranges = self.selections_in_line_number_ranges(params, line_number_ranges)?;
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

    /// First meaningful selection
    fn first(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    /// Last meaningful selection
    fn last(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    /// First selection, can be meaningless, such as empty line
    fn alpha(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    /// Last selection, can be meaningless, such as empty line
    fn omega(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>>;

    fn current(
        &self,
        params: &SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>>;

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
            .all_selections(&SelectionModeParams {
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
            .all_selections_gathered_inversely(&SelectionModeParams {
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

    fn get_paste_gap(&self, params: &SelectionModeParams, direction: &Direction) -> String {
        let buffer = params.buffer;
        let selection = params.current_selection;
        let get_in_between_gap = |direction: Direction| {
            let other = match direction {
                Direction::Start => self.delete_backward(params),
                Direction::End => self.delete_forward(params),
            }
            .ok()??;
            if other.range() == selection.range() {
                Default::default()
            } else {
                let current_range = selection.range();
                let other_range = other.range();
                let in_between_range = current_range.end.min(other_range.end)
                    ..current_range.start.max(other_range.start);
                Some(buffer.slice(&in_between_range.into()).ok()?.to_string())
            }
        };
        let prev_gap = get_in_between_gap(Direction::Start);
        let next_gap = get_in_between_gap(Direction::End);
        self.process_paste_gap(params, prev_gap, next_gap, direction)
    }

    fn process_paste_gap(
        &self,
        params: &SelectionModeParams,
        prev_gap: Option<String>,
        next_gap: Option<String>,
        direction: &Direction,
    ) -> String;
}

pub trait PositionBasedSelectionMode {
    fn selections_in_line_number_ranges(
        &self,
        params: &SelectionModeParams,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let result = line_number_ranges
            .iter()
            .map(|line_number_range| {
                self.selections_in_line_number_range(params, line_number_range.clone())
            })
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect_vec();
        // Ensure no duplicated ranges
        debug_assert!(result.iter().unique_by(|range| range.range()).count() == result.len());
        Ok(result)
    }
    fn selections_in_line_number_range(
        &self,
        params: &SelectionModeParams,
        line_number_range: Range<usize>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        Ok(line_number_range
            .map(|line_number| self.selections_in_line_number(params, line_number))
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect())
    }

    fn selections_in_line_number(
        &self,
        params: &SelectionModeParams,
        line_number: usize,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let buffer = params.buffer;
        let char_index_range = buffer.line_to_char_range(line_number)?;
        let char_index_start = char_index_range.start;

        let mut result = Vec::new();
        let mut cursor_char_index = char_index_start;
        let result = loop {
            match self.get_current_selection_by_cursor(
                buffer,
                cursor_char_index,
                IfCurrentNotFound::LookForward,
            )? {
                Some(range) => {
                    if !range_intersects(
                        &range.to_char_index_range(buffer)?.as_usize_range(),
                        &char_index_range.as_usize_range(),
                    ) {
                        break result;
                    } else {
                        let new_char_index = self.next_char_index(
                            params,
                            buffer.byte_range_to_char_index_range(range.range())?,
                        )?;
                        if new_char_index == cursor_char_index {
                            break result;
                        } else {
                            cursor_char_index = new_char_index;
                            result.push(range);
                        }
                    }
                }
                _ => {
                    break result;
                }
            }
        };
        Ok(result)
    }

    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>>;

    /// This includes all selections, including meaningless ones
    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        self.get_current_meaningful_selection_by_cursor(
            buffer,
            cursor_char_index,
            if_current_not_found,
        )
    }

    fn first(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_current_meaningful_selection_by_cursor(
            params.buffer,
            CharIndex(0),
            IfCurrentNotFound::LookForward,
        )?
        .map(|byte_range| byte_range.to_selection(params.buffer, params.current_selection))
        .transpose()
    }

    fn last(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_current_meaningful_selection_by_cursor(
            params.buffer,
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
        self.get_current_meaningful_selection_by_cursor(
            params.buffer,
            params
                .current_selection
                .range()
                .end
                .min(CharIndex(params.buffer.len_chars().saturating_sub(1))),
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
        self.get_current_meaningful_selection_by_cursor(
            params.buffer,
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

    fn alpha(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_current_selection_by_cursor(
            params.buffer,
            CharIndex(0),
            IfCurrentNotFound::LookForward,
        )?
        .map(|byte_range| byte_range.to_selection(params.buffer, params.current_selection))
        .transpose()
    }

    fn omega(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.get_current_selection_by_cursor(
            params.buffer,
            CharIndex(params.buffer.len_chars()) - 1,
            IfCurrentNotFound::LookBackward,
        )?
        .map(|byte_range| byte_range.to_selection(params.buffer, params.current_selection))
        .transpose()
    }

    fn next(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.get_current_selection_by_cursor(
            params.buffer,
            params
                .current_selection
                .range()
                .end
                .min(CharIndex(params.buffer.len_chars().saturating_sub(1))),
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

    fn previous(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.get_current_selection_by_cursor(
            params.buffer,
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
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.all_selections(params)
    }

    fn all_selections<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let mut cursor_char_index = CharIndex(0);
        let mut result = Vec::new();
        while cursor_char_index < CharIndex(params.buffer.len_chars()) {
            if let Some(range) = self.get_current_meaningful_selection_by_cursor(
                params.buffer,
                cursor_char_index,
                IfCurrentNotFound::LookForward,
            )? {
                cursor_char_index = self.next_char_index(
                    params,
                    params.buffer.byte_range_to_char_index_range(&range.range)?,
                )?;

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
        Ok(result)
    }

    #[cfg(test)]
    fn all_selections_gathered_inversely<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let mut cursor_char_index = CharIndex(params.buffer.len_chars() - 1);
        let mut result = Vec::new();
        while let Some(range) = self.get_current_meaningful_selection_by_cursor(
            params.buffer,
            cursor_char_index,
            IfCurrentNotFound::LookBackward,
        )? {
            if range.range.start == 0 || Some(&range) == result.first() {
                result.insert(0, range);
                break;
            } else {
                cursor_char_index = self.previous_char_index(
                    params,
                    params.buffer.byte_range_to_char_index_range(&range.range)?,
                )?;
                result.insert(0, range);
            }
        }
        Ok(result)
    }

    #[cfg(test)]
    fn previous_char_index(
        &self,
        _: &SelectionModeParams,
        range: CharIndexRange,
    ) -> anyhow::Result<CharIndex> {
        Ok(range.start - 1)
    }

    fn next_char_index(
        &self,
        _: &SelectionModeParams,
        range: CharIndexRange,
    ) -> anyhow::Result<CharIndex> {
        Ok(range.end)
    }

    fn expand(&self, params: &SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        params.expand()
    }

    fn up(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.vertical_movement(params, true, sticky_column_index)
    }

    fn down(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.vertical_movement(params, false, sticky_column_index)
    }

    fn vertical_movement(
        &self,
        params: &SelectionModeParams,
        is_up: bool,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        let cursor_char_index = params.cursor_char_index();
        let SelectionModeParams {
            buffer,
            current_selection,
            ..
        } = params;
        let sticky_column_index = sticky_column_index
            .or_else(|| Some(buffer.char_to_position(cursor_char_index).ok()?.column));
        let current_position = {
            let cursor_position = buffer.char_to_position(cursor_char_index)?;
            cursor_position.set_column(sticky_column_index.unwrap_or(cursor_position.column))
        };

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

        let get_column = |new_line: usize| {
            let line_length = buffer
                .get_line_by_line_index(new_line)
                .map(|slice| slice.len_chars())
                .unwrap_or_default();
            line_length.saturating_sub(1).min(current_position.column)
        };
        let mut new_position = current_position
            .set_line(new_line)
            .set_column(get_column(new_line));

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
        loop {
            if let Some(result) =
                self.get_current_selection_by_cursor(buffer, new_cursor_char_index, first_look)?
            {
                if buffer.byte_to_line(result.range.start)? == new_position.line {
                    let selection = (*current_selection)
                        .clone()
                        .set_range(buffer.byte_range_to_char_index_range(&result.range)?)
                        .set_info(result.info);
                    return Ok(Some(ApplyMovementResult {
                        selection,
                        mode: None,
                        sticky_column_index,
                    }));
                }
            }

            if let Some(result) =
                self.get_current_selection_by_cursor(buffer, new_cursor_char_index, second_look)?
            {
                if buffer.byte_to_line(result.range.start)? == new_position.line {
                    let selection = (*current_selection)
                        .clone()
                        .set_range(buffer.byte_range_to_char_index_range(&result.range)?)
                        .set_info(result.info);
                    return Ok(Some(ApplyMovementResult {
                        selection,
                        mode: None,
                        sticky_column_index,
                    }));
                }
            }

            // Move to next line
            let new_line = if is_up {
                new_position.line.saturating_sub(1)
            } else {
                new_position.line + 1
            };
            let new_column = get_column(new_line);

            new_position = Position::new(new_line, new_column);

            let next_cursor_char_index = buffer.position_to_char(new_position)?;
            if next_cursor_char_index == new_cursor_char_index {
                break;
            } else {
                new_cursor_char_index = next_cursor_char_index
            }
        }

        Ok(None)
    }

    fn current(
        &self,
        params: &SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let range = self.get_current_selection_by_cursor(
            params.buffer,
            params.cursor_char_index(),
            if_current_not_found,
        )?;
        let range = if range.is_none() {
            self.get_current_selection_by_cursor(
                params.buffer,
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
                params.buffer,
                cursor_char_index,
                IfCurrentNotFound::LookForward,
            )? {
                if current_index == index {
                    return Ok(Some(range.to_selection(buffer, current_selection)?));
                } else {
                    current_index += 1;
                    cursor_char_index = self.next_char_index(
                        params,
                        buffer.byte_range_to_char_index_range(range.range())?,
                    )?
                }
            } else {
                return Ok(None);
            }
        }
        Ok(None)
    }

    /// By default, paste gap should be empty string
    fn process_paste_gap(
        &self,
        _: &SelectionModeParams,
        _: Option<String>,
        _: Option<String>,
        _: &Direction,
    ) -> String {
        Default::default()
    }
}
pub(crate) struct PositionBased<T: PositionBasedSelectionMode>(pub(crate) T);
pub(crate) struct IterBased<T: IterBasedSelectionMode>(pub(crate) T);

impl<T: IterBasedSelectionMode> SelectionModeTrait for IterBased<T> {
    fn all_selections<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        Ok(self.0.all_meaningful_selections(params)?.collect_vec())
    }

    fn revealed_selections<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        Ok(self.0.iter_revealed(params)?.collect_vec())
    }

    #[cfg(test)]
    fn all_selections_gathered_inversely<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        Ok(self.0.all_meaningful_selections(params)?.collect_vec())
    }

    fn expand(&self, params: &SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.0.expand(params)
    }

    fn up(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.0.up(params, sticky_column_index)
    }

    fn down(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.0.down(params, sticky_column_index)
    }

    fn selections_in_line_number_ranges(
        &self,
        params: &SelectionModeParams,
        line_number_ranges: Vec<Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        self.0
            .selections_in_line_number_ranges(params, line_number_ranges)
    }

    fn next(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.next(params)
    }
    fn previous(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.previous(params)
    }

    fn to_index(
        &self,
        params: &SelectionModeParams,
        index: usize,
    ) -> anyhow::Result<Option<Selection>> {
        self.0.to_index(params, index)
    }

    fn first(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.first(params)
    }

    fn last(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.last(params)
    }

    fn current(
        &self,
        params: &SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.0.current(params, if_current_not_found)
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

    fn delete_forward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.delete_forward(params)
    }

    fn delete_backward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.delete_backward(params)
    }

    fn process_paste_gap(
        &self,
        params: &SelectionModeParams,
        prev_gap: Option<String>,
        next_gap: Option<String>,
        direction: &Direction,
    ) -> String {
        self.0
            .process_paste_gap(params, prev_gap, next_gap, direction)
    }

    fn alpha(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.alpha(params)
    }

    fn omega(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.0.omega(params)
    }
}

pub(crate) trait IterBasedSelectionMode {
    /// NOTE: this method should not be used directly,
    /// Use `iter_filtered` instead.
    /// I wish to have private trait methods :(
    fn iter<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>>;

    fn iter_filtered<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        Ok(Box::new(
            self.iter(params)?
                .chunk_by(|item| item.range.clone())
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

    fn all_meaningful_selections<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        self.iter_filtered(params)
    }

    fn all_selections<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        self.iter_filtered(params)
    }

    fn expand(&self, params: &SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
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

    fn up(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, std::cmp::Ordering::Less, sticky_column_index)
    }

    fn down(
        &self,
        params: &SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, std::cmp::Ordering::Greater, sticky_column_index)
    }

    fn select_vertical(
        &self,
        params: &SelectionModeParams,
        ordering: std::cmp::Ordering,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
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
                    position
                        .column
                        .abs_diff(sticky_column_index.unwrap_or(current_position.column)),
                )
            })
            .next()
            .map(|(_, range)| range.to_selection(buffer, current_selection))
            .transpose()?;
        Ok(selection.map(|selection| ApplyMovementResult {
            selection,
            mode: None,
            sticky_column_index: Some(sticky_column_index.unwrap_or(current_position.column)),
        }))
    }

    fn selections_in_line_number_ranges(
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
            .iter(params)?
            .filter(|range| {
                byte_ranges
                    .iter()
                    .any(|byte_range| byte_range.contains(&range.range.start))
            })
            .collect())
    }

    fn iter_revealed<'a>(
        &'a self,
        params: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        self.iter_filtered(params)
    }

    fn to_index(
        &self,
        params: &SelectionModeParams,
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

    fn right(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
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

    fn left(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        let current_selection = params.current_selection.clone();
        let buffer = params.buffer;
        let byte_range = buffer.char_index_range_to_byte_range(current_selection.range())?;
        let cursor_char_index = current_selection.to_char_index(params.cursor_direction);
        let cursor_byte = buffer.char_to_byte(cursor_char_index)?;

        Ok(self
            .iter_filtered(params)?
            .sorted()
            .rev()
            .find(|range| {
                range.range.start < cursor_byte
                    || (range.range.start == cursor_byte
                        && (range.range.start == byte_range.start
                            && range.range.end < byte_range.end))
            })
            .and_then(|range| range.to_selection(buffer, &current_selection).ok()))
    }
    fn delete_forward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.right(params)
    }

    fn delete_backward(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.left(params)
    }

    /// This uses `all_selections` instead of `iter_filtered`.
    fn first(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .all_meaningful_selections(params)?
            .sorted()
            .next()
            .and_then(|range| {
                range
                    .to_selection(params.buffer, params.current_selection)
                    .ok()
            }))
    }

    /// This uses `all_selections` instead of `iter_filtered`.
    fn last(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .all_meaningful_selections(params)?
            .sorted()
            .last()
            .and_then(|range| {
                range
                    .to_selection(params.buffer, params.current_selection)
                    .ok()
            }))
    }

    fn alpha(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .all_selections(params)?
            .sorted()
            .next()
            .and_then(|range| {
                range
                    .to_selection(params.buffer, params.current_selection)
                    .ok()
            }))
    }

    fn omega(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self
            .all_selections(params)?
            .sorted()
            .last()
            .and_then(|range| {
                range
                    .to_selection(params.buffer, params.current_selection)
                    .ok()
            }))
    }

    fn next(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.right(params)
    }

    fn previous(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        self.left(params)
    }

    fn current(
        &self,
        params: &SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.current_default_impl(params, if_current_not_found)
    }

    fn current_default_impl(
        &self,
        params: &SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<Selection>> {
        let current_selection = params.current_selection;
        let buffer = params.buffer;
        let range = current_selection.range().trimmed(buffer)?;

        if let Some((_, best_intersecting_match)) = {
            let char_index = match params.cursor_direction {
                Direction::Start => range.start,
                Direction::End => range.end - 1,
            };
            let cursor_line = buffer.char_to_line(char_index)?;
            let cursor_byte = buffer.char_to_byte(char_index)?;
            self.iter_filtered(params)?
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
        // If no intersecting match found, look in the given direction
        else {
            let result = match if_current_not_found {
                IfCurrentNotFound::LookForward => self.right(params),
                IfCurrentNotFound::LookBackward => self.left(params),
            }?;
            if let Some(result) = result {
                Ok(Some(result))
            } else {
                // Look in another direction if matching selection is not found in the
                // preferred direction
                match if_current_not_found {
                    IfCurrentNotFound::LookForward => self.left(params),
                    IfCurrentNotFound::LookBackward => self.right(params),
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
            .all_meaningful_selections(&SelectionModeParams {
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

    /// By default, paste gap should be empty string
    fn process_paste_gap(
        &self,
        _: &SelectionModeParams,
        _: Option<String>,
        _: Option<String>,
        _: &Direction,
    ) -> String {
        Default::default()
    }
}

#[cfg(test)]
mod test_selection_mode {
    use std::ops::Range;

    use crate::{
        buffer::Buffer,
        char_index_range::CharIndexRange,
        components::{
            editor::{Direction, IfCurrentNotFound, Movement, MovementApplicandum},
            suggestive_editor::Info,
        },
        selection::{CharIndex, Selection},
        selection_mode::{IterBased, SelectionModeTrait},
    };

    use super::{ByteRange, IterBasedSelectionMode, SelectionModeParams};
    use pretty_assertions::assert_eq;

    struct Dummy;
    impl IterBasedSelectionMode for Dummy {
        fn iter<'a>(
            &'a self,
            _: &SelectionModeParams<'a>,
        ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
            Ok(Box::new(
                [(0..6), (1..6), (2..5), (3..4), (3..5)]
                    .into_iter()
                    .map(ByteRange::new),
            ))
        }
    }

    fn test(
        movement: MovementApplicandum,
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
        let actual = IterBased(Dummy)
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
        test(MovementApplicandum::Left, 1..6, 0..6);
        test(MovementApplicandum::Left, 2..5, 1..6);
        test(MovementApplicandum::Left, 3..5, 3..4);

        test(MovementApplicandum::Left, 3..4, 2..5);
    }

    #[test]
    fn next() {
        test(MovementApplicandum::Right, 0..6, 1..6);
        test(MovementApplicandum::Right, 1..6, 2..5);
        test(MovementApplicandum::Right, 2..5, 3..4);
        test(MovementApplicandum::Right, 3..4, 3..5);
    }

    #[test]
    fn first() {
        test(MovementApplicandum::First, 0..1, 0..6);
    }

    #[test]
    fn last() {
        test(MovementApplicandum::Last, 0..0, 3..5);
    }

    #[test]
    fn current() {
        test(
            MovementApplicandum::Current(IfCurrentNotFound::LookForward),
            0..1,
            0..6,
        );
        test(
            MovementApplicandum::Current(IfCurrentNotFound::LookForward),
            5..6,
            1..6,
        );
        test(
            MovementApplicandum::Current(IfCurrentNotFound::LookForward),
            1..2,
            1..6,
        );
        test(
            MovementApplicandum::Current(IfCurrentNotFound::LookForward),
            3..3,
            3..4,
        );
    }

    #[test]
    fn to_index() {
        let current = 0..0;
        test(MovementApplicandum::Index(0), current.clone(), 0..6);
        test(MovementApplicandum::Index(1), current, 1..6)
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
        impl IterBasedSelectionMode for Dummy {
            fn iter<'a>(
                &'a self,
                _: &super::SelectionModeParams<'a>,
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
            let actual = IterBased(Dummy)
                .apply_movement(&params, movement.into_movement_applicandum(&None))
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
        let actual = IterBased(Dummy)
            .apply_movement(&params, MovementApplicandum::Right)
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
