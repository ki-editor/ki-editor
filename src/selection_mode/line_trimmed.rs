use crate::{components::editor::IfCurrentNotFound, selection::CharIndex};

use super::{
    ByteRange, PositionBased, PositionBasedSelectionMode, SelectionMode, SelectionModeParams,
};

#[derive(Clone)]
pub(crate) struct LineTrimmed;

impl PositionBasedSelectionMode for LineTrimmed {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        _: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let last_cursor_char_index = CharIndex(buffer.len_chars());
        if cursor_char_index >= last_cursor_char_index {
            return Ok(None);
        }
        let line_index = buffer.char_to_line(cursor_char_index)?;
        let Some(line) = buffer.get_line_by_line_index(line_index) else {
            return Ok(None);
        };
        let line_start_char_index = buffer.line_to_char(line_index)?;
        let leading_whitespace_count = line.chars().take_while(|c| c.is_whitespace()).count();
        if line.chars().all(|c| c.is_whitespace()) {
            let line_start_byte_index =
                buffer.char_to_byte(line_start_char_index + leading_whitespace_count - 1)?;
            return Ok(Some(ByteRange::new(
                line_start_byte_index..line_start_byte_index,
            )));
        }

        let trailing_whitespace_count = if line.len_chars() == 0 {
            0
        } else {
            (0..line.len_chars())
                .rev()
                .take_while(|index| line.char(*index).is_whitespace())
                .count()
        };
        let range = buffer.char_index_range_to_byte_range(
            (line_start_char_index + leading_whitespace_count
                ..line_start_char_index + line.len_chars() - trailing_whitespace_count)
                .into(),
        )?;
        Ok(Some(ByteRange::new(range)))
    }

    fn next_char_index(
        &self,
        params: &SelectionModeParams,
        range: crate::char_index_range::CharIndexRange,
    ) -> anyhow::Result<CharIndex> {
        let line_index = params.buffer.char_to_line(range.start)?;
        params.buffer.line_to_char(line_index + 1)
    }

    #[cfg(test)]
    fn previous_char_index(
        &self,
        params: &SelectionModeParams,
        range: crate::char_index_range::CharIndexRange,
    ) -> anyhow::Result<CharIndex> {
        let line_index = params.buffer.char_to_line(range.start)?;
        params.buffer.line_to_char(line_index - 1)
    }

    fn left(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        } = params;
        let current_line = buffer.char_to_line(current_selection.extended_range().start)?;
        Ok(buffer
            .get_parent_lines(current_line)?
            .into_iter()
            .filter(|line| line.line < current_line)
            .next_back()
            .map(|line| {
                let byte_range = buffer.line_to_byte_range(line.line)?;
                let start = trim_leading_spaces(byte_range.range.start, &line.content);
                let char_index_range =
                    buffer.byte_range_to_char_index_range(&(start..start + 1))?;
                PositionBased(self.clone()).current(
                    &SelectionModeParams {
                        buffer,
                        cursor_direction,
                        current_selection: &(**current_selection)
                            .clone()
                            .set_range(char_index_range),
                    },
                    IfCurrentNotFound::LookForward,
                )
            })
            .transpose()?
            .flatten())
    }

    fn right(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.down(params)
    }

    fn delete_forward(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        PositionBased(self.clone()).down(params)
    }

    fn delete_backward(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        PositionBased(self.clone()).up(params)
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
    use crate::{buffer::Buffer, components::editor::Direction, selection::Selection};

    use super::*;

    #[test]
    fn simple_case() {
        let buffer = Buffer::new(None, "a\n\nb");
        PositionBased(LineTrimmed).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..1, "a"), (2..2, ""), (3..4, "b")],
        );
    }

    #[test]
    fn case_1() {
        let buffer = Buffer::new(None, "a\n\n\nb  \nc\n  hello\n  \nbye\n\n");
        PositionBased(LineTrimmed).assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..1, "a"),
                (2..2, ""),
                (3..3, ""),
                (4..5, "b"),
                (8..9, "c"),
                (12..17, "hello"),
                (20..20, ""),
                (21..24, "bye"),
                (25..25, ""),
            ],
        );
    }

    #[test]
    fn single_line_without_trailing_newline_character() {
        let buffer = Buffer::new(None, "a");
        PositionBased(LineTrimmed).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..1, "a")],
        );
    }

    #[test]
    fn to_parent_line() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::LANGUAGE.into()),
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
            let result = PositionBased(LineTrimmed)
                .left(&SelectionModeParams {
                    buffer: &buffer,
                    current_selection: &Selection::new((start..start + 1).into()),
                    cursor_direction: &Direction::default(),
                })
                .unwrap()
                .unwrap();

            let actual = buffer.slice(&result.extended_range()).unwrap();
            assert_eq!(actual, expected);
        };

        test(4, "fn g() {");

        test(1, "fn f() {");
    }
}
