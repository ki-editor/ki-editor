pub mod token;

use std::ops::Range;

use crate::{buffer::Buffer, components::editor::CursorDirection, selection::Selection};

pub trait SelectionMode {
    fn iter<'a>(
        buffer: &'a Buffer,
        current_selection: Selection,
    ) -> Box<dyn Iterator<Item = Selection> + 'a>;

    // TODO: add the jumps method, which can be more efficient the current jump method, which
    // recomputes again for every jump

    fn right(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &CursorDirection,
    ) -> Option<Selection> {
        let mut iter = Self::iter(buffer, current_selection.clone());

        let cursor_char_index = current_selection.to_char_index(cursor_direction);

        while let Some(selection) = iter.next() {
            let selection_cursor_index = selection.to_char_index(cursor_direction);

            if selection_cursor_index > cursor_char_index {
                return Some(selection);
            }

            if selection_cursor_index == cursor_char_index
                && (match cursor_direction {
                    CursorDirection::Start => selection.range.end != current_selection.range.end,
                    CursorDirection::End => selection.range.start != current_selection.range.start,
                })
            {
                return Some(selection);
            }
        }
        None
    }

    fn left(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &CursorDirection,
    ) -> Option<Selection> {
        let mut iter = Self::iter(buffer, current_selection.clone());

        let mut previous_selection = None;

        let cursor_char_index = current_selection.to_char_index(cursor_direction);

        while let Some(selection) = iter.next() {
            let selection_cursor_index = selection.to_char_index(cursor_direction);

            // TODO: handle overlap
            if selection_cursor_index >= cursor_char_index {
                // log::info!("previous_selection: {:?}", previous_selection);
                return previous_selection;
            } else {
                previous_selection = Some(selection);
            }
        }

        None
    }
}
