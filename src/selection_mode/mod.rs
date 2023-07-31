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
    buffer::Buffer, components::editor::CursorDirection, position::Position, selection::Selection,
};

pub struct ByteRange(pub Range<usize>);

pub trait SelectionMode {
    fn iter<'a>(
        &'a self,
        buffer: &'a Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>>;

    // TODO: add the jumps method, which can be more efficient the current jump method, which
    // recomputes again for every jump

    fn right(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &CursorDirection,
    ) -> anyhow::Result<Option<Selection>> {
        let mut iter = self.iter(buffer)?;

        let cursor_char_index = current_selection.to_char_index(cursor_direction);

        while let Some(range) = iter.next() {
            let start = buffer.byte_to_char(range.0.start)?;
            let end = buffer.byte_to_char(range.0.end)?;
            let selection_cursor_index = match cursor_direction {
                CursorDirection::Start => start,
                CursorDirection::End => end,
            };

            if selection_cursor_index > cursor_char_index
                || (selection_cursor_index == cursor_char_index
                    && (match cursor_direction {
                        CursorDirection::Start => end != current_selection.range.end,
                        CursorDirection::End => start != current_selection.range.start,
                    }))
            {
                return Ok(Some(Selection {
                    range: start..end,
                    ..current_selection.clone()
                }));
            }
        }
        Ok(None)
    }

    fn left(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &CursorDirection,
    ) -> anyhow::Result<Option<Selection>> {
        let mut iter = self.iter(buffer)?;

        let mut previous_selection = None;

        let cursor_char_index = current_selection.to_char_index(cursor_direction);

        while let Some(range) = iter.next() {
            let start = buffer.byte_to_char(range.0.start)?;
            let end = buffer.byte_to_char(range.0.end)?;
            let selection_cursor_index = match cursor_direction {
                CursorDirection::Start => start,
                CursorDirection::End => end,
            };

            // TODO: handle overlap
            if selection_cursor_index >= cursor_char_index {
                // log::info!("previous_selection: {:?}", previous_selection);
                return Ok(previous_selection);
            } else {
                previous_selection = Some(Selection {
                    range: start..end,
                    ..current_selection.clone()
                });
            }
        }

        Ok(None)
    }

    /// By default this means the next selection after the current selection which is on the next
    /// line and of the same column
    fn down(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &CursorDirection,
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
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &CursorDirection,
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
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &CursorDirection,
    ) -> anyhow::Result<Option<Selection>> {
        let mut iter = self.iter(buffer)?;

        let cursor_char_index = current_selection.to_char_index(cursor_direction);

        while let Some(range) = iter.next() {
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
