use crate::char_index_range::CharIndexRange;

use super::IterBasedSelectionMode;

pub(crate) struct Mark {
    pub(crate) marks: Vec<CharIndexRange>,
}

impl IterBasedSelectionMode for Mark {
    fn iter<'a>(
        &'a self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        Ok(Box::new(self.marks.iter().filter_map(|range| {
            let start = buffer.char_to_byte(range.start).ok()?;
            let end = buffer.char_to_byte(range.end).ok()?;
            Some(super::ByteRange::new(start..end))
        })))
    }
}
