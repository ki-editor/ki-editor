use super::SelectionMode;

pub struct Line;

impl SelectionMode for Line {
    fn iter<'a>(
        &'a self,
        _current_selection: &'a crate::selection::Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let len_lines = buffer.len_lines();

        Ok(Box::new((0..len_lines).filter_map(move |line_index| {
            let start = buffer.line_to_char(line_index).ok()?;
            let line = buffer.get_line(start).ok()?;
            let start = buffer.char_to_byte(start).ok()?;
            let end = start + line.len_chars();

            Some(super::ByteRange::new(start..end))
        })))
    }
}

#[cfg(test)]
mod test_line {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a\n\nb\nc\n");
        Line.assert_all_selections(&buffer, Selection::default(), &[]);
    }
}
