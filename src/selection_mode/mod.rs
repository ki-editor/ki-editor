pub mod ast_grep;
pub mod custom;
pub mod diagnostic;
pub mod largest_node;
pub mod line;
pub mod node;
pub mod regex;
pub mod token;

pub use self::regex::Regex;
pub use ast_grep::AstGrep;
pub use custom::Custom;
pub use diagnostic::Diagnostic;
use itertools::Itertools;
pub use largest_node::LargestNode;
pub use line::Line;
pub use node::Node;
pub use token::Token;

use std::ops::Range;

use crate::{
    buffer::Buffer,
    components::editor::{CursorDirection, Direction, Jump},
    position::Position,
    selection::{CharIndex, Selection},
};

#[derive(PartialEq, Eq)]
pub struct ByteRange(pub Range<usize>);
impl ByteRange {
    fn to_char_index_range(&self, buffer: &Buffer) -> anyhow::Result<Range<CharIndex>> {
        Ok(buffer.byte_to_char(self.0.start)?..buffer.byte_to_char(self.0.end)?)
    }

    fn to_byte(&self, cursor_direction: &CursorDirection) -> usize {
        match cursor_direction {
            CursorDirection::Start => self.0.start,
            CursorDirection::End => self.0.end,
        }
    }
}

impl PartialOrd for ByteRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0
            .start
            .partial_cmp(&other.0.start)
            .or_else(|| self.0.end.partial_cmp(&other.0.end))
    }
}

impl Ord for ByteRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .start
            .cmp(&other.0.start)
            .then(self.0.end.cmp(&other.0.end))
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
        buffer: &'a Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>>;

    fn jumps(
        &self,
        params: SelectionModeParams,
        chars: Vec<char>,
        direction: &Direction,
    ) -> anyhow::Result<Vec<Jump>> {
        let iter = match direction {
            Direction::Left => self.left_iter(&params)?,
            _ => self.right_iter(&params)?,
        };
        Ok(chars
            .into_iter()
            .zip(iter)
            .filter_map(|(character, range)| {
                Some(Jump {
                    character,
                    selection: Selection {
                        range: range.to_char_index_range(params.buffer).ok()?,
                        ..params.current_selection.clone()
                    },
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
        let iter = self.iter(buffer)?;
        let cursor_byte = buffer.char_to_byte(current_selection.to_char_index(cursor_direction))?;
        let cursor_direction = (*cursor_direction).clone();
        Ok(Box::new(iter.sorted().filter(move |range| {
            range.to_byte(&cursor_direction) > cursor_byte
        })))
    }

    fn right(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self.right_iter(&params)?.next().and_then(|range| {
            Some(Selection {
                range: range.to_char_index_range(params.buffer).ok()?,
                ..params.current_selection.clone()
            })
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
        Ok(self
            .right_iter(&params)?
            .filter(|range| range.0.start <= current_line_range.0.end)
            .last()
            .map(|range| Selection {
                range: buffer.byte_to_char(range.0.start).unwrap()
                    ..buffer.byte_to_char(range.0.end).unwrap(),
                ..current_selection.clone()
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
        let iter = self.iter(buffer)?;
        let cursor_byte = buffer.char_to_byte(current_selection.to_char_index(cursor_direction))?;
        Ok(Box::new(
            iter.sorted_by(|a, b| b.cmp(a))
                .filter(move |range| range.0.start < cursor_byte),
        ))
    }

    fn left(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self.left_iter(&params)?.next().and_then(|range| {
            Some(Selection {
                range: range.to_char_index_range(params.buffer).ok()?,
                ..params.current_selection.clone()
            })
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
        Ok(self
            .left_iter(&params)?
            .filter(|range| current_line_range.0.start <= range.0.start)
            .last()
            .map(|range| Selection {
                range: buffer.byte_to_char(range.0.start).unwrap()
                    ..buffer.byte_to_char(range.0.end).unwrap(),
                ..current_selection.clone()
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
            .iter(buffer)?
            .filter_map(|range| {
                let start = buffer.byte_to_char(range.0.start).ok()?;
                let end = buffer.byte_to_char(range.0.end).ok()?;

                let selection_position = buffer.char_to_position(start).ok()?;

                if filter_fn(selection_position) {
                    Some((start..end, selection_position))
                } else {
                    None
                }
            })
            .sorted_by_key(|(_, position)| {
                let column_diff = position.column as i64 - cursor_position.column as i64;
                let line_diff = position.line as i64 - cursor_position.line as i64;

                (line_diff.abs(), column_diff.abs())
            })
            .next();

        Ok(found.map(|(range, _)| Selection {
            range,
            ..current_selection.clone()
        }))
    }

    fn current(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        let iter = self.iter(buffer)?;

        let cursor_char_index = current_selection.to_char_index(cursor_direction);

        for range in iter {
            let start = buffer.byte_to_char(range.0.start)?;
            let end = buffer.byte_to_char(range.0.end)?;

            if start == cursor_char_index {
                return Ok(Some(Selection {
                    range: start..end,
                    ..current_selection.clone()
                }));
            }

            if start > cursor_char_index {
                break;
            }
        }

        let iter = self.iter(buffer)?;
        let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
        let found = iter
            .filter(|range| range.0.contains(&cursor_byte))
            .sorted_by_key(|range| range.0.end - range.0.start)
            .next();

        if let Some(found) = found {
            return Ok(Some(Selection {
                range: buffer.byte_to_char(found.0.start)?..buffer.byte_to_char(found.0.end)?,
                ..current_selection.clone()
            }));
        }

        Ok(None)
    }
}
