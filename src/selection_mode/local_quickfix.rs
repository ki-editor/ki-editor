use std::rc::Rc;

use super::{ByteRange, VectorBased, VectorBasedSelectionMode};

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

impl VectorBasedSelectionMode for LocalQuickfix {
    fn get_byte_ranges(&self, _: &crate::buffer::Buffer) -> anyhow::Result<Rc<Vec<ByteRange>>> {
        Ok(self.ranges.clone())
    }
}
