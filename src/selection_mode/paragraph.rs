use crate::{components::editor::IfCurrentNotFound, selection::CharIndex};

use super::{ByteRange, PositionBasedSelectionMode};

#[derive(Clone)]
pub struct Paragraph;

fn is_empty_line(buffer: &crate::buffer::Buffer, line_index: usize) -> bool {
    buffer
        .get_line_by_line_index(line_index)
        .map(|line| line.chars().all(|c| c.is_whitespace()))
        .unwrap_or(true)
}

/// Get the byte range of the paragraph containing `line_index`.
/// A paragraph is a contiguous run of non-empty lines.
/// The range spans from the start of the first line to the end of the last line
fn paragraph_byte_range_for_line(
    buffer: &crate::buffer::Buffer,
    line_index: usize,
) -> anyhow::Result<Option<ByteRange>> {
    if is_empty_line(buffer, line_index) {
        return Ok(None);
    }

    // Expand upward to find start of paragraph
    let mut start_line = line_index;
    while start_line > 0 && !is_empty_line(buffer, start_line - 1) {
        start_line -= 1;
    }

    // Expand downward to find end of paragraph
    let mut end_line = line_index;
    let len_lines = buffer.len_lines();
    while end_line + 1 < len_lines && !is_empty_line(buffer, end_line + 1) {
        end_line += 1;
    }

    let start_char = buffer.line_to_char(start_line)?;

    let end_char = {
        let end_line_start = buffer.line_to_char(end_line)?;
        let end_line_content = buffer.get_line_by_line_index(end_line)?;
        let end_line_char_count = end_line_content.chars().count();
        end_line_start + end_line_char_count
    };

    let byte_range = buffer.char_index_range_to_byte_range((start_char..end_char).into())?;
    Ok(Some(ByteRange::new(byte_range)))
}

impl PositionBasedSelectionMode for Paragraph {
    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        if buffer.len_chars() == 0 {
            return Ok(None);
        }
        let cursor_char_index =
            cursor_char_index.min(CharIndex(buffer.len_chars().saturating_sub(1)));
        let line_index = buffer.char_to_line(cursor_char_index)?;

        if let Some(range) = paragraph_byte_range_for_line(buffer, line_index)? {
            return Ok(Some(range));
        }

        // If cursor is on an empty line, look in the requested direction
        match if_current_not_found {
            IfCurrentNotFound::LookForward => {
                let len_lines = buffer.len_lines();
                for i in (line_index + 1)..len_lines {
                    if let Some(range) = paragraph_byte_range_for_line(buffer, i)? {
                        return Ok(Some(range));
                    }
                }
                Ok(None)
            }
            IfCurrentNotFound::LookBackward => {
                for i in (0..line_index).rev() {
                    if let Some(range) = paragraph_byte_range_for_line(buffer, i)? {
                        return Ok(Some(range));
                    }
                }
                Ok(None)
            }
        }
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: CharIndex,
        _: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        if buffer.len_chars() == 0 {
            return Ok(None);
        }
        let cursor_char_index =
            cursor_char_index.min(CharIndex(buffer.len_chars().saturating_sub(1)));
        let line_index = buffer.char_to_line(cursor_char_index)?;

        if let Some(range) = paragraph_byte_range_for_line(buffer, line_index)? {
            return Ok(Some(range));
        }

        // Cursor is on an empty line — return it as a zero-width selection at line start
        let line_start = buffer.line_to_char(line_index)?;
        let byte_range = buffer.char_index_range_to_byte_range((line_start..line_start).into())?;
        Ok(Some(ByteRange::new(byte_range)))
    }
}

#[cfg(test)]
mod test_paragraph {
    use crate::{
        buffer::Buffer,
        selection::Selection,
        selection_mode::{PositionBased, SelectionModeTrait},
        test_app::execute_test,
    };

    use crate::buffer::BufferOwner;
    use crate::components::editor::Movement;
    use crate::selection::SelectionMode;
    use crate::test_app::*;

    use super::*;

    #[test]
    fn basic_paragraphs() {
        // "foo\nbar" is a paragraph, empty line separates, "baz\nqux" is another paragraph
        let buffer = Buffer::new(None, "foo\nbar\n\nbaz\nqux");
        PositionBased(super::Paragraph).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..8, "foo\nbar\n"), (9..16, "baz\nqux")],
        );
    }

    #[test]
    fn single_paragraph_no_empty_lines() {
        let buffer = Buffer::new(None, "hello\nworld");
        PositionBased(super::Paragraph).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..11, "hello\nworld")],
        );
    }

    #[test]
    fn single_line() {
        let buffer = Buffer::new(None, "hello");
        PositionBased(super::Paragraph).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..5, "hello")],
        );
    }

    #[test]
    fn multiple_empty_lines_between_paragraphs() {
        // "foo\n\n\nbar": foo=0..3, empty lines at 3,4,5, bar=6..9
        let buffer = Buffer::new(None, "foo\n\n\nbar");
        PositionBased(super::Paragraph).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..4, "foo\n"), (6..9, "bar")],
        );
    }

    #[test]
    fn paragraphs_navigation_using_left_right() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
foo
bar

spam
baz
"
                    .to_string(),
                )),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Paragraph,
                )),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Movement::Right)),
                Expect(CurrentSelectedTexts(&["foo\nbar\n"])),
                Editor(MoveSelection(Movement::Right)),
                Expect(CurrentSelectedTexts(&["spam\nbaz\n"])),
            ])
        })
    }
}
