use crate::selection::Selection;

use super::{IterBasedSelectionMode, SelectionMode};

pub(crate) struct Custom {
    current_selection: Selection,
}

impl Custom {
    pub(crate) fn new(current_selection: Selection) -> Custom {
        Custom { current_selection }
    }
}

impl IterBasedSelectionMode for Custom {
    fn iter<'a>(
        &'a self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        let range = self.current_selection.extended_range();
        Ok(Box::new(std::iter::once(super::ByteRange::new(
            buffer.char_to_byte(range.start)?..buffer.char_to_byte(range.end)?,
        ))))
    }
}
