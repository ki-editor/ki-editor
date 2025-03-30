use super::{ByteRange, SelectionMode};

// TODO: change this to custom selections, so it can also hold references, definitions etc
pub(crate) struct LocalQuickfix {
    ranges: Vec<ByteRange>,
}

impl LocalQuickfix {
    pub(crate) fn new(params: super::SelectionModeParams<'_>) -> Self {
        let buffer = params.buffer;
        let ranges = buffer
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
            .collect();
        Self { ranges }
    }
}

impl SelectionMode for LocalQuickfix {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
        Ok(self
            .ranges
            .iter()
            .find(|range| range.range.contains(&cursor_byte))
            .cloned())
    }
}
