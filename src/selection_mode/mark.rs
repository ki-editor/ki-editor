use crate::buffer::Buffer;

use super::{
    get_current_selection_by_cursor_via_iter, ByteRange, PositionBasedSelectionMode, VectorBased,
    VectorBasedSelectionMode,
};
use std::rc::Rc;

pub(crate) struct Mark {
    ranges: Rc<Vec<ByteRange>>,
}

impl Mark {
    pub(crate) fn new(buffer: &Buffer) -> anyhow::Result<Self> {
        Ok(Mark {
            ranges: Rc::new(
                buffer
                    .marks()
                    .into_iter()
                    .filter_map(|range| {
                        Some(ByteRange::new(
                            buffer.char_index_range_to_byte_range(range).ok()?,
                        ))
                    })
                    .collect(),
            ),
        })
    }
}

impl VectorBasedSelectionMode for Mark {
    fn get_byte_ranges(&self, buffer: &Buffer) -> anyhow::Result<Rc<Vec<ByteRange>>> {
        Ok(self.ranges.clone())
    }
}
