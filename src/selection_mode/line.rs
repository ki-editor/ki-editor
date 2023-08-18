use super::SelectionMode;

pub struct Line;

impl SelectionMode for Line {
    fn iter<'a>(
        &'a self,
        _current_selection: &'a crate::selection::Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        // Need to sub 1, because apparently it will always return one more line
        let len_lines = buffer.rope().len_lines().saturating_sub(1);

        Ok(Box::new((0..len_lines).filter_map(move |line_index| {
            let start = buffer.line_to_byte(line_index).ok()?;
            let end = buffer.line_to_byte(line_index + 1).ok()?.saturating_sub(1);

            Some(super::ByteRange::new(start..end))
        })))
    }
}
