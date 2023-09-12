use super::SelectionMode;

pub struct Bookmark;

impl SelectionMode for Bookmark {
    fn name(&self) -> &'static str {
        "BOOKMARK"
    }
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        Ok(Box::new(buffer.bookmarks().into_iter().filter_map(
            |range| {
                let start = buffer.char_to_byte(range.start).ok()?;
                let end = buffer.char_to_byte(range.end).ok()?;
                Some(super::ByteRange::new(start..end))
            },
        )))
    }
}
