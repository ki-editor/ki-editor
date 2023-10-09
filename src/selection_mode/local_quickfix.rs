use crate::components::suggestive_editor::Info;

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
            .and_then(|path| params.context.get_quickfix_items(&path))
            .map(|items| {
                items
                    .into_iter()
                    .filter_map(|item| {
                        Some(super::ByteRange::with_info(
                            buffer
                                .position_range_to_byte_range(&item.location().range)
                                .ok()?,
                            Info::new(item.infos().join("\n\n")),
                        ))
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
