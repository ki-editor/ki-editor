use crate::selection::Selection;

use super::{ByteRange, PositionBased, PositionBasedSelectionMode};

pub(crate) struct Custom {
    current_selection: Selection,
}

impl Custom {
    pub(crate) fn new(current_selection: Selection) -> Custom {
        Custom { current_selection }
    }
}

impl PositionBasedSelectionMode for Custom {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        _: crate::selection::CharIndex,
        _: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        let range = self.current_selection.extended_range();
        let byte_range = buffer.char_index_range_to_byte_range(range)?;
        Ok(Some(
            ByteRange::new(byte_range).set_info(self.current_selection.info()),
        ))
    }
}
