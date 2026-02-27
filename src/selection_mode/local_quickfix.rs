use crate::quickfix_list::QuickfixListItem;

use super::{ByteRange, IterBasedSelectionMode};

// TODO: change this to custom selections, so it can also hold references, definitions etc
pub struct LocalQuickfix {
    ranges: Vec<ByteRange>,
}

impl LocalQuickfix {
    pub fn new(
        params: super::SelectionModeParams<'_>,
        quickfix_list_items: &[QuickfixListItem],
    ) -> Self {
        let buffer = params.buffer;
        let ranges = quickfix_list_items
            .iter()
            .filter_map(|item| {
                if Some(&item.location().path) != buffer.path().as_ref() {
                    None
                } else {
                    Some(
                        super::ByteRange::new(
                            buffer
                                .char_index_range_to_byte_range(item.location().range)
                                .ok()?,
                        )
                        .set_info(item.info().clone()),
                    )
                }
            })
            .collect();
        Self { ranges }
    }
}

impl IterBasedSelectionMode for LocalQuickfix {
    fn iter<'a>(
        &'a self,
        _: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(self.ranges.clone().into_iter()))
    }
}
