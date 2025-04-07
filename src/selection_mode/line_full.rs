use crate::selection::CharIndex;

use super::{ByteRange, PositionBasedSelectionMode};

pub(crate) struct LineFull;

impl LineFull {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl PositionBasedSelectionMode for LineFull {
    fn right(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let start_char_index = {
            let cursor_char_index = params.cursor_char_index();

            // If current line is already an empty line,
            // find the next group of empty lines
            if buffer
                .get_line_by_char_index(cursor_char_index)?
                .chars()
                .all(|char| char.is_whitespace())
            {
                let mut index = cursor_char_index;
                loop {
                    if index > CharIndex(buffer.len_chars().saturating_sub(1)) {
                        return Ok(None);
                    } else if buffer.char(index).is_whitespace() {
                        index = index + 1
                    } else {
                        break index;
                    }
                }
            } else {
                cursor_char_index
            }
        };
        let mut line_index = buffer.char_to_line(start_char_index)?;

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
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let start_char_index = {
            let cursor_char_index = params.cursor_char_index();

            // If current line is already an empty line,
            // find the previous group of empty lines
            if buffer
                .get_line_by_char_index(cursor_char_index)?
                .chars()
                .all(|char| char.is_whitespace())
            {
                let mut index = cursor_char_index;
                loop {
                    if buffer.char(index).is_whitespace() {
                        if index == CharIndex(0) {
                            return Ok(None);
                        } else {
                            index = index - 1
                        }
                    } else {
                        break index;
                    }
                }
            } else {
                cursor_char_index
            }
        };
        let mut line_index = buffer.char_to_line(start_char_index)?;
        while let Some(slice) = buffer.get_line_by_line_index(line_index) {
            if slice.chars().all(|char| char.is_whitespace()) {
                let range = buffer.line_to_char_range(line_index)?;
                return Ok(Some(params.current_selection.clone().set_range(range)));
            } else if line_index == 0 {
                break;
            } else {
                line_index -= 1
            }
        }
        Ok(None)
    }

    fn delete_forward(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.down(params)
    }

    fn delete_backward(
        &self,
        params: &super::SelectionModeParams,
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

#[cfg(test)]
mod test_line_full {
    use crate::{
        buffer::Buffer,
        selection::Selection,
        selection_mode::{PositionBased, SelectionMode as _},
    };

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(None, "a\n\n\nb\nc\n  hello");
        PositionBased(LineFull).assert_all_selections(
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
        PositionBased(LineFull).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..1, "a")],
        );
    }
}
