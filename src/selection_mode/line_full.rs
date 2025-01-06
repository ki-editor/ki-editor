use itertools::Itertools;

use super::SelectionMode;

pub(crate) struct LineFull;

impl SelectionMode for LineFull {
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        let len_lines = buffer.len_lines();

        Ok(Box::new(
            (0..len_lines)
                .take(
                    // This is a weird hack, because `rope.len_lines`
                    // returns an extra line which is empty if the rope ends with the newline character
                    if buffer.rope().to_string().ends_with('\n') {
                        len_lines.saturating_sub(1)
                    } else {
                        len_lines
                    },
                )
                .filter_map(move |line_index| {
                    let line = buffer.get_line_by_line_index(line_index)?;
                    let start = buffer.line_to_byte(line_index).ok()?;
                    let len_bytes = line.len_bytes();
                    let end = start + len_bytes;

                    Some(super::ByteRange::new(start..end))
                }),
        ))
    }
    fn right<'a>(
        &self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let current_selection_range =
            buffer.char_index_range_to_byte_range(current_selection.range())?;
        Ok(self
            .iter_filtered(params)?
            .skip_while(|byte_range| byte_range.range != current_selection_range)
            .skip_while(|byte_range| is_blank(buffer, byte_range).unwrap_or(false))
            .find_map(|byte_range| {
                if is_blank(buffer, &byte_range)? {
                    buffer
                        .byte_range_to_char_index_range(&byte_range.range)
                        .ok()
                } else {
                    None
                }
            })
            .map(|range| current_selection.clone().set_range(range)))
    }

    fn left<'a>(
        &self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let current_selection_range =
            buffer.char_index_range_to_byte_range(current_selection.range())?;
        Ok(self
            .iter_filtered(params)?
            .take_while(|byte_range| byte_range.range != current_selection_range)
            .collect_vec()
            .into_iter()
            .rev()
            .skip_while(|byte_range| is_blank(buffer, byte_range).unwrap_or(false))
            .find_map(|byte_range| {
                if is_blank(buffer, &byte_range)? {
                    buffer
                        .byte_range_to_char_index_range(&byte_range.range)
                        .ok()
                } else {
                    None
                }
            })
            .map(|range| current_selection.clone().set_range(range)))
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
}

fn is_blank(buffer: &crate::buffer::Buffer, byte_range: &super::ByteRange) -> Option<bool> {
    let range = buffer
        .byte_range_to_char_index_range(&byte_range.range)
        .ok()?;
    let content = buffer.slice(&range).ok()?;
    Some(content.chars().all(|c| c.is_whitespace()))
}

#[cfg(test)]
mod test_line {
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
