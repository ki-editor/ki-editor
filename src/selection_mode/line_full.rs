use crate::{components::editor::IfCurrentNotFound, selection::CharIndex};

use crate::selection_mode::ApplyMovementResult;

use super::{ByteRange, PositionBasedSelectionMode};

pub struct LineFull;

impl PositionBasedSelectionMode for LineFull {
    fn up(
        &self,
        params: &super::SelectionModeParams,
        _sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        let buffer = params.buffer;
        let start_char_index = {
            let cursor_char_index = params
                .cursor_char_index()
                .min(CharIndex(buffer.len_chars()) - 1);

            // If current line is already an empty line,
            // find the previous group of empty lines
            if buffer
                .get_line_by_char_index(cursor_char_index)?
                .chars()
                .all(|char| char.is_whitespace())
            {
                let mut index = cursor_char_index;
                loop {
                    if buffer.char(index)?.is_whitespace() {
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
                return Ok(self
                    .get_current_selection_by_cursor(
                        params.buffer,
                        buffer.line_to_char(line_index)?,
                        IfCurrentNotFound::LookBackward,
                    )?
                    .and_then(|byte_range| {
                        Some(ApplyMovementResult::from_selection(
                            params.current_selection.clone().set_range(
                                buffer
                                    .byte_range_to_char_index_range(byte_range.range())
                                    .ok()?,
                            ),
                        ))
                    }));
            } else if line_index == 0 {
                break;
            } else {
                line_index -= 1
            }
        }
        Ok(None)
    }

    fn down(
        &self,
        params: &super::SelectionModeParams,
        _sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
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
                    } else if buffer.char(index)?.is_whitespace() {
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
                    return Ok(self
                        .to_index(params, line_index)?
                        .map(ApplyMovementResult::from_selection));
                } else {
                    line_index += 1
                }
            } else {
                break;
            }
        }
        Ok(None)
    }

    fn get_current_meaningful_selection_by_cursor(
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
        selection_mode::{PositionBased, SelectionModeTrait as _},
    };

    

    #[test]
    fn case_1() {
        let buffer = Buffer::new(None, "a\n\n\nb\nc\n  hello");
        PositionBased(super::LineFull).assert_all_selections(
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
        PositionBased(super::LineFull).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..1, "a")],
        );
    }
}
