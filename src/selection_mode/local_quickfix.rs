use super::{ByteRange, SelectionMode};

// TODO: change this to custom selections, so it can also hold references, definitions etc
pub struct LocalQuickfix {
    ranges: Vec<ByteRange>,
}

impl LocalQuickfix {
    pub fn new(params: super::SelectionModeParams<'_>) -> Self {
        let buffer = params.buffer;
        let ranges = params
            .buffer
            .path()
            .map(|path| buffer.quickfix_list_items())
            .map(|items| {
                items
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
                    .collect()
            })
            .unwrap_or_default();
        Self { ranges }
    }
}

impl SelectionMode for LocalQuickfix {
    fn name(&self) -> &'static str {
        "LOCAL QUICKFIX"
    }
    fn iter<'a>(
        &'a self,
        _: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(self.ranges.clone().into_iter()))
    }
}
