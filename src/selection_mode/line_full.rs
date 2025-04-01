use itertools::Itertools;

use super::{ByteRange, SelectionMode};

pub(crate) struct LineFull;

impl SelectionMode for LineFull {
    fn right(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let mut line_index = buffer.char_to_line(params.cursor_char_index())?;
        while line_index < buffer.len_lines() {
            if let Some(slice) = buffer.get_line_by_line_index(line_index) {
                if slice.chars().all(|char| char.is_whitespace()) {
                    let range = buffer.line_to_char_range(line_index)?;
                    return Ok(Some(params.current_selection.clone().set_range(range)));
                } else {
                    line_index += 1
                }
            } else {
                break;
            }
        }
        Ok(None)
    }

    fn left(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let mut line_index = buffer.char_to_line(params.cursor_char_index())?;
        loop {
            if let Some(slice) = buffer.get_line_by_line_index(line_index) {
                if slice.chars().all(|char| char.is_whitespace()) {
                    let range = buffer.line_to_char_range(line_index)?;
                    return Ok(Some(params.current_selection.clone().set_range(range)));
                } else if line_index == 0 {
                    break;
                } else {
                    line_index -= 1
                }
            } else {
                break;
            }
        }
        Ok(None)
    }

    fn delete_forward(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.down(params)
    }

    fn delete_backward(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.up(params)
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        _: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let line_index = buffer.char_to_line(cursor_char_index)?;
        let line_start_char_index = buffer.line_to_char(line_index)?;
        let Some(line) = buffer.get_line_by_line_index(line_index) else {
            return Ok(None);
        };
        let range = buffer.char_index_range_to_byte_range(
            (line_start_char_index..line_start_char_index + line.len_chars()).into(),
        )?;
        Ok(Some(ByteRange::new(range)))
    }
}

fn is_blank(buffer: &crate::buffer::Buffer, byte_range: &super::ByteRange) -> Option<bool> {
    let range = buffer
        .byte_range_to_char_index_range(&byte_range.range)
        .ok()?;
    let content = buffer.slice(&range).ok()?;
    Some(content.chars().all(|c| c.is_whitespace()))
}

#[cfg(test)]
mod test_line_full {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(None, "a\n\n\nb\nc\n  hello");
        LineFull.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                // Each selection should have trailing newline character
                (0..2, "a\n"),
                (2..3, "\n"),
                (3..4, "\n"),
                (4..6, "b\n"),
                (6..8, "c\n"),
                // Should include leading whitespaces
                (8..15, "  hello"),
            ],
        );
    }

    #[test]
    fn single_line_without_trailing_newline_character() {
        let buffer = Buffer::new(None, "a");
        LineFull.assert_all_selections(&buffer, Selection::default(), &[(0..1, "a")]);
    }
}
