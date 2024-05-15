use super::{LineFull, SelectionMode};
use crate::selection_mode::ApplyMovementResult;

pub(crate) struct LineTrimmed;

impl SelectionMode for LineTrimmed {
    fn name(&self) -> &'static str {
        "LINE(TRIMMED)"
    }
    fn parent(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        Ok(LineFull
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
                    let end = start
                        + if line.to_string().ends_with('\n') {
                            len_bytes.saturating_sub(1)
                        } else {
                            len_bytes
                        };
                    let start = trim_leading_spaces(start, &line.to_string()).min(end);
                    Some(super::ByteRange::new(start..end))
                }),
        ))
    }
}

pub(crate) fn trim_leading_spaces(byte_start: usize, line: &str) -> usize {
    if line == "\n" {
        byte_start
    } else {
        let leading_whitespace_count = line
            .to_string()
            .chars()
            .take_while(|c| c.is_whitespace())
            .count();
        byte_start.saturating_add(leading_whitespace_count)
    }
}

#[cfg(test)]
mod test_line {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(None, "a\n\n\nb\nc\n  hello\n  \nbye");
        LineTrimmed.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..1, "a"),
                (2..2, ""),
                (3..3, ""),
                (4..5, "b"),
                (6..7, "c"),
                // Should not include leading whitespaces
                (10..15, "hello"),
                (18..18, ""),
                (19..22, "bye"),
            ],
        );
    }

    #[test]
    fn single_line_without_trailing_newline_character() {
        let buffer = Buffer::new(None, "a");
        LineTrimmed.assert_all_selections(&buffer, Selection::default(), &[(0..1, "a")]);
    }
}
