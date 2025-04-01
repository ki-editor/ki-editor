use std::rc::Rc;

use super::{get_current_selection_by_cursor_via_iter, ByteRange, SelectionMode};

// TODO: change this to custom selections, so it can also hold references, definitions etc
pub(crate) struct LocalQuickfix {
    ranges: Rc<Vec<ByteRange>>,
}

impl LocalQuickfix {
    pub(crate) fn new(params: super::SelectionModeParams<'_>) -> Self {
        let buffer = params.buffer;
        let ranges = Rc::new(
            buffer
                .quickfix_list_items()
                .into_iter()
                .filter_map(|item| {
                    Some(
                        super::ByteRange::new(
                            buffer
                                .position_range_to_byte_range(&item.location().range)
                                .ok()?,
                        )
                        .set_info(item.info().clone()),
                    )
                })
                .collect(),
        );
        Self { ranges }
    }
}

impl SelectionMode for LocalQuickfix {
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
