use crate::components::editor::IfCurrentNotFound;
use crate::selection::{CharIndex, Selection};
use itertools::Either;
use ropey::Rope;

use super::{ByteRange, PositionBasedSelectionMode};

pub struct Character;

struct CurrentLine {
    line: Rope,
    first_char_index: CharIndex,
}

impl CurrentLine {
    fn from_params(params: &super::SelectionModeParams) -> anyhow::Result<Self> {
        let buffer = params.buffer;
        let cursor = params
            .current_selection
            .to_char_index(params.cursor_direction);
        let line_number = buffer.char_to_line(cursor)?;
        let first_char_index = buffer.line_to_char(line_number)?;
        let line = buffer.get_line_by_char_index(cursor)?;
        Ok(Self {
            line,
            first_char_index,
        })
    }

    fn char_at(
        &self,
        params: &super::SelectionModeParams,
        offset: usize,
    ) -> anyhow::Result<Option<Selection>> {
        char_index_to_selection(
            params.buffer,
            params.current_selection,
            self.first_char_index + offset,
        )
    }

    fn first_non_whitespace(
        &self,
        params: &super::SelectionModeParams,
        reversed: bool,
    ) -> anyhow::Result<Option<Selection>> {
        let line_len = self.line.len_chars();
        let mut indices = if reversed {
            Either::Left((0..line_len).rev())
        } else {
            Either::Right(0..line_len)
        };
        Ok(indices
            .find(|&i| !self.line.char(i).is_whitespace())
            .map(|index| self.char_at(params, index))
            .transpose()?
            .flatten())
    }
}

impl PositionBasedSelectionMode for Character {
    fn first(&self, params: &super::SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        CurrentLine::from_params(params)?.char_at(params, 0)
    }

    fn last(&self, params: &super::SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        let current_line = CurrentLine::from_params(params)?;
        let line_len = current_line.line.len_chars();
        if line_len == 0 {
            return Ok(None);
        }
        current_line.char_at(params, line_len - 1)
    }

    fn previous(&self, params: &super::SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        CurrentLine::from_params(params)?.first_non_whitespace(params, false)
    }

    fn next(&self, params: &super::SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        CurrentLine::from_params(params)?.first_non_whitespace(params, true)
    }

    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        _: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        let Some(last_char_index) = buffer.last_char_index() else {
            return Ok(None);
        };
        let cursor_char_index = cursor_char_index.min(last_char_index);
        let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
        let char = buffer.char(cursor_char_index)?;
        Ok(Some(ByteRange::new(
            cursor_byte..cursor_byte + char.len_utf8(),
        )))
    }
}

fn char_index_to_selection(
    buffer: &crate::buffer::Buffer,
    current_selection: &Selection,
    char_index: CharIndex,
) -> anyhow::Result<Option<Selection>> {
    let byte_start = buffer.char_to_byte(char_index)?;
    let ch = buffer.char(char_index)?;
    let byte_range = ByteRange::new(byte_start..byte_start + ch.len_utf8());
    Ok(Some(byte_range.to_selection(buffer, current_selection)?))
}

#[cfg(test)]
mod test_character {
    use crate::app::Dimension;
    use crate::buffer::BufferOwner;
    use crate::components::editor::Movement;
    use crate::selection::SelectionMode;
    use crate::selection_mode::{PositionBased, SelectionModeTrait};
    use crate::test_app::*;

    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() -> anyhow::Result<()> {
        let buffer = Buffer::new(None, "foo\nspam");

        // First line
        let selection = Selection::default();
        crate::selection_mode::SelectionModeTrait::assert_all_selections(
            &PositionBased(super::Character),
            &buffer,
            selection,
            &[
                (0..1, "f"),
                (1..2, "o"),
                (2..3, "o"),
                (3..4, "\n"),
                (4..5, "s"),
                (5..6, "p"),
                (6..7, "a"),
                (7..8, "m"),
            ],
        );

        // Second line
        let char_index = buffer.line_to_char(1)?;
        let selection = Selection::default().set_range((char_index..char_index).into());
        crate::selection_mode::SelectionModeTrait::assert_all_selections(
            &PositionBased(super::Character),
            &buffer,
            selection,
            &[
                (0..1, "f"),
                (1..2, "o"),
                (2..3, "o"),
                (3..4, "\n"),
                (4..5, "s"),
                (5..6, "p"),
                (6..7, "a"),
                (7..8, "m"),
            ],
        );
        Ok(())
    }

    #[test]
    fn last_char_of_file_should_not_exceed_bound() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("f".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Character,
                )),
                Expect(CurrentSelectedTexts(&["f"])),
                Editor(MoveSelection(Down)),
                Expect(CurrentSelectedTexts(&["f"])),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["f"])),
            ])
        })
    }

    #[test]
    fn empty_buffer_should_not_be_character_selectable() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Character,
                )),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Left)),
                Expect(CurrentSelectedTexts(&[""])),
            ])
        })
    }

    #[test]
    fn multiwidth_unicode_char() {
        let buffer = Buffer::new(None, "大學之道");
        PositionBased(super::Character).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..3, "大"), (3..6, "學"), (6..9, "之"), (9..12, "道")],
        );
    }

    #[test]
    fn jump_char() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo\nbar\nspam".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Character,
                )),
                App(TerminalDimensionChanged(Dimension {
                    height: 10,
                    width: 10,
                })),
                Editor(ShowJumps {
                    use_current_selection_mode: true,
                    prior_change: None,
                }),
                Expect(JumpChars(&[
                    '\n', '\n', 'a', 'a', 'b', 'f', 'm', 'o', 'o', 'p', 'r', 's',
                ])),
            ])
        })
    }

    #[test]
    fn first_last_of_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo\nbar".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Character,
                )),
                // Cursor starts at 'f'
                Expect(CurrentSelectedTexts(&["f"])),
                // First of line -> 'f' (already at first)
                Editor(MoveSelection(Movement::First)),
                Expect(CurrentSelectedTexts(&["f"])),
                // Last of line -> '\n'
                Editor(MoveSelection(Movement::Last)),
                Expect(CurrentSelectedTexts(&["\n"])),
                // Move to second line
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["b"])),
                // First of second line -> 'b'
                Editor(MoveSelection(Movement::First)),
                Expect(CurrentSelectedTexts(&["b"])),
                // Last of second line -> 'm'
                Editor(MoveSelection(Movement::Last)),
                Expect(CurrentSelectedTexts(&["r"])),
            ])
        })
    }

    #[test]
    fn previous_next_non_whitespace() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("  foo  \n  bar  ".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Character,
                )),
                // Cursor starts at first char ' '
                Expect(CurrentSelectedTexts(&[" "])),
                // Previous (first non-whitespace of line) -> 'f'
                Editor(MoveSelection(Movement::Previous)),
                Expect(CurrentSelectedTexts(&["f"])),
                // Next (last non-whitespace of line) -> 'o'
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["o"])),
                // Move to second line
                Editor(MoveSelection(Down)),
                // Previous (first non-whitespace of second line) -> 'b'
                Editor(MoveSelection(Movement::Previous)),
                Expect(CurrentSelectedTexts(&["b"])),
                // Next (last non-whitespace of second line) -> 'r'
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["r"])),
            ])
        })
    }
}
