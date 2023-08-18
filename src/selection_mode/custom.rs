use crate::selection::Selection;

use super::SelectionMode;

pub struct Custom {
    current_selection: Selection,
}

impl Custom {
    pub fn new(current_selection: Selection) -> Custom {
        Custom { current_selection }
    }
}

impl SelectionMode for Custom {
    fn iter<'a>(
        &'a self,
        _current_selection: &'a Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let range = self.current_selection.range();
        Ok(Box::new(std::iter::once(super::ByteRange::new(
            buffer.char_to_byte(range.start)?..buffer.char_to_byte(range.end)?,
        ))))
    }
}
