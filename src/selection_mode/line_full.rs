use super::{LineTrimmed, SelectionMode};
use crate::selection_mode::ApplyMovementResult;

pub(crate) struct LineFull;

impl SelectionMode for LineFull {
    fn first_child(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        Ok(LineTrimmed
            .current(params)?
            .map(ApplyMovementResult::from_selection))
    }
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
