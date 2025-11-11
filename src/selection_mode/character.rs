use crate::components::editor::IfCurrentNotFound;

use super::{
    subword::SelectionPosition, ByteRange, PositionBased, PositionBasedSelectionMode,
    SelectionModeTrait, Subword,
};

pub(crate) struct Character;

impl PositionBasedSelectionMode for Character {
    fn first(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_char(params, SelectionPosition::First)
    }

    fn last(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_char(params, SelectionPosition::Last)
    }

    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        _: IfCurrentNotFound,
        _: crate::char_index_range::CharIndexRange,
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

fn get_char(
    params: &super::SelectionModeParams,
    position: SelectionPosition,
) -> anyhow::Result<Option<crate::selection::Selection>> {
    if let Some(current_word) = PositionBased(Subword::new()).current(
        params,
        crate::components::editor::IfCurrentNotFound::LookForward,
    )? {
        let start = match position {
            SelectionPosition::First => current_word.range().start,
            SelectionPosition::Last => current_word.range().end - 1,
        };
        return Ok(Some(
            params
                .current_selection
                .clone()
                .set_range((start..start + 1).into()),
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod test_character {
    use crate::app::Dimension;
    use crate::buffer::BufferOwner;
    use crate::selection::SelectionMode;
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
}
