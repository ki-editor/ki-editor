use super::SelectionMode;

pub struct Line;

impl SelectionMode for Line {
    fn name(&self) -> &'static str {
        "LINE"
    }
    fn iter<'a>(
        &'a self,
        _current_selection: &'a crate::selection::Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let len_lines = buffer.len_lines().saturating_sub(1);

        Ok(Box::new((0..len_lines).filter_map(move |line_index| {
            let line = buffer.get_line_by_line_index(line_index)?;
            let start = buffer.line_to_byte(line_index).ok()?;
            let end = start + line.len_bytes();

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
        let buffer = Buffer::new(tree_sitter_rust::language(), "a\n\n\nb\nc\n");
        Line.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..2, "a\n"),
                (2..3, "\n"),
                (3..4, "\n"),
                (4..6, "b\n"),
                (6..8, "c\n"),
            ],
        );
    }
}
