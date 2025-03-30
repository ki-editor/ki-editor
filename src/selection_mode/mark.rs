use crate::char_index_range::CharIndexRange;

use super::SelectionMode;

pub(crate) struct Mark;

impl SelectionMode for Mark {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        Ok(buffer.marks().iter().find_map(|range| {
            if range.contains(&cursor_char_index) {
                Some(super::ByteRange::new(
                    buffer.char_index_range_to_byte_range(*range).ok()?,
                ))
            } else {
                None
            }
        }))
    }
}
