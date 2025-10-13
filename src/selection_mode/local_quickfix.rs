use super::{ByteRange, IterBasedSelectionMode};

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
                            .char_index_range_to_byte_range(item.location().range)
                            .ok()?,
                    )
                    .set_info(item.info().clone()),
                )
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
