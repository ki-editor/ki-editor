use super::{get_current_selection_by_cursor_via_iter, ByteRange, SelectionMode};
use std::rc::Rc;

pub(crate) struct Mark {
    pub(crate) ranges: Rc<Vec<ByteRange>>,
}

impl SelectionMode for Mark {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        get_current_selection_by_cursor_via_iter(
            buffer,
            cursor_char_index,
            if_current_not_found,
            self.ranges.clone(),
        )
    }
}
