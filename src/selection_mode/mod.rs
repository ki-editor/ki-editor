pub mod ast_grep;
pub mod bookmark;
pub mod custom;
pub mod diagnostic;
pub mod git_hunk;
pub mod largest_node;
pub mod line;
pub mod regex;
pub mod syntax_tree;
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
pub use syntax_tree::SyntaxTree;
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
        Ok(selection.clone().set_range(self.to_char_index_range(buffer)?).set_info(self.info))
    }
}

impl PartialOrd for ByteRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.range
            .start
            .partial_cmp(&other.range.start)
            .or_else(|| self.range.end.partial_cmp(&other.range.end))
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

    fn right_iter<'a>(
        &'a self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        }: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let iter = self.iter(current_selection, buffer)?;
        let cursor_byte = buffer.char_to_byte(current_selection.to_char_index(cursor_direction))?;
        let cursor_direction = (*cursor_direction).clone();
        Ok(Box::new(iter.sorted().filter(move |range| {
            range.to_byte(&cursor_direction) > cursor_byte
        })))
    }

    fn right(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self.right_iter(&params)?.next().and_then(|range| {
            range
                .to_selection(params.buffer, params.current_selection)
                .ok()
        }))
    }

    fn right_most(
        &self,
        params @ SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        let current_line_range =
            buffer.current_line_byte_range(current_selection.to_char_index(cursor_direction))?;
        Ok(self.right_iter(&params)?.last().and_then(|range| {
            range
                .to_selection(params.buffer, params.current_selection)
                .ok()
        }))
    }

    fn left_iter<'a>(
        &'a self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        }: &SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let iter = self.iter(current_selection, buffer)?;
        let cursor_byte = buffer.char_to_byte(current_selection.to_char_index(cursor_direction))?;
        Ok(Box::new(
            iter.sorted_by(|a, b| b.cmp(a))
                .filter(move |range| range.range.start < cursor_byte),
        ))
    }

    fn left(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self.left_iter(&params)?.next().and_then(|range| {
            range
                .to_selection(params.buffer, params.current_selection)
                .ok()
        }))
    }

    fn left_most(
        &self,
        params @ SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        let current_line_range =
            buffer.current_line_byte_range(current_selection.to_char_index(cursor_direction))?;
        Ok(self.left_iter(&params)?.last().and_then(|range| {
            range
                .to_selection(params.buffer, params.current_selection)
                .ok()
        }))
    }

    /// By default this means the next selection after the current selection which is on the next
    /// line and of the same column
    fn down(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        self.move_vertically(buffer, current_selection, cursor_direction, false)
    }

    /// Default implementation:
    ///
    /// Get the selection that is at least one line above the current selection,
    /// and the column is the nearest to that of the current selection, regardless of left or
    /// right.
    fn up(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        self.move_vertically(buffer, current_selection, cursor_direction, true)
    }

    fn move_vertically(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &CursorDirection,
        go_up: bool,
    ) -> anyhow::Result<Option<Selection>> {
        let cursor_position =
            buffer.char_to_position(current_selection.to_char_index(cursor_direction))?;

        let filter_fn = move |selection_position: Position| {
            if go_up {
                selection_position.line < cursor_position.line
            } else {
                selection_position.line > cursor_position.line
            }
        };

        let found = self
            .iter(current_selection, buffer)?
            .filter_map(|range| {
                let start = buffer.byte_to_char(range.range.start).ok()?;
                let end = buffer.byte_to_char(range.range.end).ok()?;

                let selection_position = buffer.char_to_position(start).ok()?;

                if filter_fn(selection_position) {
                    Some(((start..end).into(), range.info, selection_position))
                } else {
                    None
                }
            })
            .sorted_by_key(|(_, _, position)| {
                let column_diff = position.column as i64 - cursor_position.column as i64;
                let line_diff = position.line as i64 - cursor_position.line as i64;

                (line_diff.abs(), column_diff.abs())
            })
            .next();

        Ok(found.map(|(range, info, _)| current_selection.clone().set_range(range).set_info(info)))
    }

    fn current(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        let cursor_char_index = current_selection.to_char_index(cursor_direction);
        let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
        if let Some(exact) = self
            .iter(current_selection, buffer)?
            .find(|range| range.range.start == cursor_byte)
        {
            return exact.to_selection(buffer, current_selection).map(Some);
        }

        let found = self
            .iter(current_selection, buffer)?
            .filter(|range| range.range.contains(&cursor_byte))
            .sorted_by_key(|range| range.range.end - range.range.start)
            .next();

        Ok(found.and_then(|range| range.to_selection(buffer, current_selection).ok()))
    }
}
