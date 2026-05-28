use std::ops::Not;

use crate::{
    char_index_range::CharIndexRange,
    components::editor::IfCurrentNotFound,
    selection::{CharIndex, Selection},
};

use super::{ByteRange, PositionBasedSelectionMode};

#[derive(Clone)]
pub struct LineTrimmed;

impl PositionBasedSelectionMode for LineTrimmed {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        if cursor_char_index > CharIndex(buffer.len_chars()) {
            return Ok(None);
        }

        let line_index = buffer.char_to_line(cursor_char_index)?;
        let line = buffer.get_line_by_line_index(line_index)?;
        let current_trimmed_line_range = {
            let line = line.to_string().trim_end_matches('\n').to_string();
            let head_trimmed_line: String =
                line.chars().skip_while(|c| c.is_whitespace()).collect();
            let leading_whitespaces_count = line
                .chars()
                .count()
                .saturating_sub(head_trimmed_line.chars().count());

            let trailing_whitespaces_count = head_trimmed_line
                .chars()
                .rev()
                .take_while(|c| c.is_whitespace())
                .count();
            let cursor_line_start = buffer.line_to_char(line_index)?;
            let start = cursor_line_start + leading_whitespaces_count;

            CharIndexRange::from(
                start
                    ..start + line.chars().count()
                        - leading_whitespaces_count
                        - trailing_whitespaces_count,
            )
        };

        // Determine whether the cursor falls before, within or after the current_trimmed_line_range
        let within_range = if current_trimmed_line_range.is_empty() {
            current_trimmed_line_range.start == cursor_char_index
        } else {
            current_trimmed_line_range.start <= cursor_char_index
                && cursor_char_index < current_trimmed_line_range.end
        };

        let result = Ok(Some(ByteRange::new(
            buffer.char_index_range_to_byte_range(current_trimmed_line_range)?,
        )));
        if within_range {
            // Cursor falls within range
            result
        } else if cursor_char_index < current_trimmed_line_range.start {
            // Cursor falls before range
            match if_current_not_found {
                IfCurrentNotFound::LookBackward => {
                    let next_cursor_char_index = cursor_char_index - 1;
                    if cursor_char_index == next_cursor_char_index {
                        return Ok(None);
                    }
                    // Recursively decrement cursor_char_index
                    // until it lands in a trimmed line range
                    self.get_current_selection_by_cursor(
                        buffer,
                        next_cursor_char_index,
                        if_current_not_found,
                    )
                }
                IfCurrentNotFound::LookForward => result,
            }
        } else {
            // Cursor falls after range
            match if_current_not_found {
                IfCurrentNotFound::LookBackward => result,
                IfCurrentNotFound::LookForward => {
                    // Recursively increment cursor_char_index
                    // until it lands in a trimmed line range
                    self.get_current_selection_by_cursor(
                        buffer,
                        cursor_char_index + 1,
                        if_current_not_found,
                    )
                }
            }
        }
    }

    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        get_line(buffer, cursor_char_index, if_current_not_found)
    }

    fn next(&self, params: &super::SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        scan_for_empty_line(self, params, IfCurrentNotFound::LookForward)
    }

    fn previous(&self, params: &super::SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        scan_for_empty_line(self, params, IfCurrentNotFound::LookBackward)
    }
}

fn scan_for_empty_line(
    selection_mode: &LineTrimmed,
    params: &super::SelectionModeParams,
    if_current_not_found: IfCurrentNotFound,
) -> anyhow::Result<Option<Selection>> {
    let buffer = params.buffer;
    let current_line_index = buffer.char_to_line(params.cursor_char_index())?;
    let lines: Box<dyn Iterator<Item = usize>> = match if_current_not_found {
        IfCurrentNotFound::LookForward => Box::new((current_line_index + 1)..buffer.len_chars()),
        IfCurrentNotFound::LookBackward => Box::new((0..current_line_index).rev()),
    };
    for line_index in lines {
        let Ok(slice) = buffer.get_line_by_line_index(line_index) else {
            break;
        };
        if slice.chars().all(|char| char.is_whitespace()) {
            return Ok(selection_mode
                .get_current_selection_by_cursor(
                    params.buffer,
                    buffer.line_to_char(line_index)?,
                    if_current_not_found,
                )?
                .and_then(|byte_range| {
                    Some(
                        params.current_selection.clone().set_range(
                            buffer
                                .byte_range_to_char_index_range(byte_range.range())
                                .ok()?,
                        ),
                    )
                }));
        }
    }
    Ok(None)
}

fn get_line(
    buffer: &crate::buffer::Buffer,
    cursor_char_index: crate::selection::CharIndex,
    if_current_not_found: crate::components::editor::IfCurrentNotFound,
) -> anyhow::Result<Option<super::ByteRange>> {
    if buffer.len_chars() == 0 {
        return Ok(None);
    }
    let (start_index, end_index) = {
        let cursor_char_index = cursor_char_index.min(CharIndex(buffer.len_chars()) - 1);

        let is_skippable = |c: char| c.is_whitespace();

        let cursor_char_index = match if_current_not_found {
            IfCurrentNotFound::LookForward => {
                let mut index = cursor_char_index;
                let len_chars = buffer.len_chars().saturating_sub(1);
                loop {
                    let ch = buffer.char(index)?;
                    if is_skippable(ch).not() {
                        break index;
                    } else if index.0 == len_chars {
                        return Ok(None);
                    } else {
                        index = index + 1;
                    }
                }
            }
            IfCurrentNotFound::LookBackward => {
                let mut index = cursor_char_index;
                loop {
                    let ch = buffer.char(index)?;
                    if is_skippable(ch).not() {
                        break index;
                    } else if index.0 == 0 {
                        return Ok(None);
                    } else {
                        index = index - 1;
                    }
                }
            }
        };

        let mut left_index = cursor_char_index;

        let mut left_most_non_whitespace = cursor_char_index;
        let start_index = loop {
            if left_index == CharIndex(0) {
                break left_most_non_whitespace;
            }
            left_index = left_index - 1;
            let Ok(ch) = buffer.char(left_index) else {
                break left_most_non_whitespace;
            };
            if ch == '\n' {
                break left_most_non_whitespace;
            } else if ch.is_whitespace() {
                continue;
            } else {
                left_most_non_whitespace = left_index;
            }
        };

        let end_index = {
            let mut right_encountered_non_whitespace = false;
            let mut right_last_non_whitespace = CharIndex(0);
            let mut right_index = left_most_non_whitespace;
            loop {
                if right_index.0 == buffer.len_chars() {
                    break;
                }

                right_index = right_index + 1;
                let Ok(ch) = buffer.char(right_index) else {
                    break;
                };
                if ch == '\n' {
                    break;
                } else if ch.is_whitespace() {
                    continue;
                } else {
                    right_encountered_non_whitespace = true;
                    right_last_non_whitespace = right_index;
                }
            }
            if right_encountered_non_whitespace {
                right_last_non_whitespace + 1
            } else {
                cursor_char_index + 1
            }
        };

        (start_index, end_index)
    };
    let trimmed_range = buffer.char_index_range_to_byte_range((start_index..end_index).into())?;
    Ok(Some(ByteRange::new(trimmed_range)))
}

#[cfg(test)]
mod test_line {
    use crate::buffer::BufferOwner;
    use crate::components::editor::{Direction, Movement};
    use crate::position::Position;
    use crate::selection::SelectionMode;
    use crate::test_app::*;

    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    use serial_test::serial;

    use crate::selection_mode::{GetGapMovement, PositionBased, SelectionModeTrait};

    #[test]
    fn left_right_movement() {
        let buffer = Buffer::new(None, "a\n\nb");
        PositionBased(LineTrimmed).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..1, "a"), (3..4, "b")],
        );
    }

    #[test]
    fn jump_to_line_number() {
        let buffer = Buffer::new(
            None,
            "foo
bar
spam

baz",
        );
        let result = PositionBased(LineTrimmed)
            .to_index(
                &crate::selection_mode::SelectionModeParams {
                    buffer: &buffer,
                    current_selection: &Selection::default(),
                    cursor_direction: &Direction::End,
                },
                4,
            )
            .unwrap()
            .unwrap();
        let selection = buffer.slice(&result.range()).unwrap();
        assert_eq!(selection, "baz");
    }

    #[test]
    fn up_down_movement() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(" a \n \nb".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Expect(CurrentSelectedTexts(&["a"])),
                Editor(MoveSelection(Movement::Down)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Movement::Down)),
                Expect(CurrentSelectedTexts(&["b"])),
                Editor(MoveSelection(Movement::Up)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Movement::Up)),
                Expect(CurrentSelectedTexts(&["a"])),
            ])
        })
    }

    #[test]
    fn case_1() {
        let buffer = Buffer::new(None, "a\n\n\nb  \nc\n  hello\n  \nbye\n\n");
        PositionBased(LineTrimmed).assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..1, "a"),
                (4..5, "b"),
                (8..9, "c"),
                (12..17, "hello"),
                (21..24, "bye"),
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

    #[serial]
    #[test]
    fn paste_forward_use_larger_indent() -> anyhow::Result<()> {
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
"
                    .trim()
                    .to_string(),
                )),
                Editor(MatchLiteral("bar".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(Copy),
                Editor(PasteWithMovement(GetGapMovement::Right)),
                Expect(CurrentComponentContent(
                    "
foo
  bar
    bar
    spam
"
                    .trim(),
                )),
            ])
        })
    }

    #[serial]
    #[test]
    fn paste_previous_using_last_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo\nbar".to_string())),
                Editor(MatchLiteral("bar".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(Copy),
                Editor(PasteWithMovement(GetGapMovement::Left)),
                Expect(CurrentComponentContent("foo\nbar\nbar")),
            ])
        })
    }

    #[test]
    fn able_to_go_to_last_line_which_is_empty() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello\n".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(MoveSelection(Movement::Next)),
                Expect(EditorCursorPosition(crate::position::Position {
                    line: 1,
                    column: 0,
                })),
                Expect(CurrentSelectedTexts(&[""])),
            ])
        })
    }

    #[test]
    fn able_to_delete_forward_at_last_line_which_is_empty() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello\n".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(MoveSelection(Movement::Last)),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(DeleteWithMovement(Movement::Right)),
                Expect(CurrentComponentContent("hello")),
            ])
        })
    }

    #[test]
    fn able_to_delete_backward_at_last_line_which_is_empty() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello\n".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(MoveSelection(Movement::Last)),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(DeleteWithMovement(Left)),
                Expect(CurrentComponentContent("hello")),
            ])
        })
    }

    #[test]
    fn able_to_move_right_left_on_unicode_lines() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
[÷] 🦀  main.rs
1│fn first () {
5│  █ifth();
6│}
"
                    .trim()
                    .to_string(),
                )),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Expect(CurrentSelectedTexts(&["[÷] 🦀  main.rs"])),
                Editor(MoveSelection(Movement::Right)),
                Expect(CurrentSelectedTexts(&["1│fn first () {"])),
                Editor(MoveSelection(Movement::Right)),
                Expect(CurrentSelectedTexts(&["5│  █ifth();"])),
                Editor(MoveSelection(Movement::Left)),
                Expect(CurrentSelectedTexts(&["1│fn first () {"])),
            ])
        })
    }

    #[test]
    fn able_to_move_prev_when_at_last_empty_line() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
world

hello
"
                    .to_string(),
                )),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(MoveSelection(Movement::Last)),
                Expect(CurrentSelectedTexts(&["hello"])),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&[""])),
                Expect(ExpectKind::EditorCursorPosition(Position::new(4, 0))),
                Editor(MoveSelection(Movement::Previous)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Movement::Left)),
                Expect(CurrentSelectedTexts(&["world"])),
            ])
        })
    }

    #[test]
    fn cursor_on_empty_space_between_words() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello world\nbye".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                Editor(MatchLiteral("world".to_string())),
                Editor(EnterInsertMode(Direction::Start)),
                Editor(EnterNormalMode),
                Expect(CurrentSelectedTexts(&[" "])),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Expect(CurrentSelectedTexts(&["hello world"])),
            ])
        })
    }

    #[test]
    fn empty_line_navigation_using_up_down() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo\n\nbar".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(MoveSelection(Movement::Down)),
                Expect(EditorCursorPosition(Position::new(1, 0))),
                Editor(MoveSelection(Movement::Down)),
                Expect(EditorCursorPosition(Position::new(2, 0))),
                Editor(MoveSelection(Movement::Up)),
                Expect(EditorCursorPosition(Position::new(1, 0))),
            ])
        })
    }
}
