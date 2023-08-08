use super::SelectionMode;

pub struct Bookmark;

impl SelectionMode for Bookmark {
    fn iter<'a>(
        &'a self,
        current_selection: &'a crate::selection::Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(buffer.bookmarks().into_iter().filter_map(
            |range| {
                let start = buffer.char_to_byte(range.start).ok()?;
                let end = buffer.char_to_byte(range.end).ok()?;
                Some(super::ByteRange::new(start..end))
            },
        )))
    }
}
