use super::{ApplyMovementResult, ByteRange, SelectionMode};

pub struct Line;

impl SelectionMode for Line {
    fn name(&self) -> &'static str {
        "LINE"
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
                    let start = trim_leading_spaces(start, &line.to_string());

                    Some(super::ByteRange::new(start..end))
                }),
        ))
    }
    fn up(
        &self,
        super::SelectionModeParams {
            buffer,
            current_selection,
            ..
        }: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        let current_line = buffer.char_to_line(current_selection.extended_range().start)?;
        Ok(buffer
            .get_parent_lines(current_line)?
            .into_iter()
            .filter(|line| line.line < current_line)
            .next_back()
            .map(|line| {
                let byte_range = buffer.line_to_byte_range(line.line)?;
                let start = trim_leading_spaces(byte_range.range.start, &line.content);
                ByteRange {
                    range: start..byte_range.range.end,
                    ..byte_range
                }
                .to_selection(buffer, current_selection)
            })
            .transpose()?
            .map(ApplyMovementResult::from_selection))
    }
}

fn trim_leading_spaces(byte_start: usize, line: &str) -> usize {
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
    use crate::{
        buffer::Buffer,
        components::editor::Direction,
        context::Context,
        selection::{Filters, Selection},
        selection_mode::SelectionModeParams,
    };

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a\n\n\nb\nc\n  hello");
        Line.assert_all_selections(
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
            ],
        );
    }

    #[test]
    fn single_line_without_trailing_newline_character() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a");
        Line.assert_all_selections(&buffer, Selection::default(), &[(0..1, "a")]);
    }

    #[test]
    fn up() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "
fn f() {
    fn g() {
        let a = 1;
        let b = 2;
        let c = 3;
        let d = 4;
    }

}"
            .trim(),
        );

        let test = |selected_line: usize, expected: &str| {
            let start = buffer.line_to_char(selected_line).unwrap();
            let result = Line
                .up(SelectionModeParams {
                    buffer: &buffer,
                    current_selection: &Selection::new((start..start + 1).into()),
                    cursor_direction: &Direction::default(),
                    context: &Context::default(),
                    filters: &Filters::default(),
                })
                .unwrap()
                .unwrap()
                .selection;

            let actual = buffer.slice(&result.extended_range()).unwrap();
            assert_eq!(actual, expected);
        };

        test(4, "fn g() {");

        test(1, "fn f() {");
    }
}
