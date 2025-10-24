use itertools::Itertools;

use crate::{
    components::editor::{Direction, IfCurrentNotFound},
    selection::CharIndex,
};

use super::{ByteRange, PositionBasedSelectionMode, SelectionModeParams};

#[derive(Clone)]
pub(crate) struct LineTrimmed;

impl PositionBasedSelectionMode for LineTrimmed {
    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let max_cursor_char_index = CharIndex(buffer.len_chars());
        if cursor_char_index > max_cursor_char_index {
            return Ok(None);
        }

        // adjust for first go (for now it is the main logic)
        let portion = get_portion(buffer, cursor_char_index);
        let mut current_line_index = cursor_char_index.to_line(buffer)?;
        current_line_index = match if_current_not_found {
            IfCurrentNotFound::LookForward => match portion {
                Portion::Leading => current_line_index,
                Portion::Trimmed => current_line_index,
                Portion::Trailing => current_line_index.saturating_add(1),
            },
            IfCurrentNotFound::LookBackward => match portion {
                Portion::Leading => current_line_index.saturating_sub(1),
                Portion::Trimmed => current_line_index,
                Portion::Trailing => current_line_index,
            },
        };

        // find target line (for now it will just break)
        let is_target = |_: ropey::RopeSlice| true;
        loop {
            let Some(current_line) = buffer.get_line_by_line_index(current_line_index) else {
                return Ok(None);
            };
            if is_target(current_line) {
                break;
            }
            current_line_index = match if_current_not_found {
                IfCurrentNotFound::LookForward => current_line_index.saturating_add(1),
                IfCurrentNotFound::LookBackward => current_line_index.saturating_sub(1),
            }
        }

        trimmed_range(buffer, current_line_index)
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        _: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let max_cursor_char_index = CharIndex(buffer.len_chars());
        if cursor_char_index > max_cursor_char_index {
            return Ok(None);
        }
        let line_index = cursor_char_index.to_line(buffer)?;

        let Some(line) = buffer.get_line_by_line_index(line_index) else {
            return Ok(None);
        };
        let line_portions = get_line_portions(line);
        let line_start = buffer.line_to_char(line_index)?;
        let line_end = line_start + line.len_chars();

        let portion = get_portion(buffer, cursor_char_index);
        match portion {
            Portion::Leading => {
                let range = buffer.char_index_range_to_byte_range(
                    (line_start..line_start + line_portions.leading).into(),
                )?;
                Ok(Some(ByteRange::new(range)))
            }
            Portion::Trimmed => trimmed_range(buffer, line_index),
            Portion::Trailing => {
                let range = buffer.char_index_range_to_byte_range(
                    (line_end - line_portions.trailing..line_end).into(),
                )?;
                Ok(Some(ByteRange::new(range)))
            }
        }
    }

    fn process_paste_gap(
        &self,
        params: &super::SelectionModeParams,
        prev_gap: Option<String>,
        next_gap: Option<String>,
        direction: &crate::components::editor::Direction,
    ) -> String {
        process_paste_gap(params, prev_gap, next_gap, direction)
    }
}

enum Portion {
    Leading,
    Trimmed,
    Trailing,
}

struct LinePortions {
    leading: usize,
    trimmed: usize,
    trailing: usize,
}

fn get_portion(buffer: &crate::buffer::Buffer, cursor_char_index: CharIndex) -> Portion {
    let line_index = cursor_char_index.to_line(buffer).unwrap();
    let line = buffer.get_line_by_line_index(line_index).unwrap();
    let line_start = buffer.line_to_char(line_index).unwrap();
    //let line_end = line_start + line.len_chars();

    let line_portions = get_line_portions(line);
    let char_position = cursor_char_index.0 - line_start.0;
    //debug_assert!(char_postion >= 0);
    //debug_assert!(char_postion <= line.len_chars());

    if char_position < line_portions.leading {
        Portion::Leading
    } else if char_position < line_portions.leading + line_portions.trimmed {
        Portion::Trimmed
    } else {
        Portion::Trailing
    }
}

fn get_line_portions(line: ropey::RopeSlice) -> LinePortions {
    let leading = line
        .chars()
        .take_while(|c| c.is_whitespace() && c != &'\n')
        .count();
    let trimmed = line.to_string().trim().len();
    let trailing = (leading..line.len_chars())
        .rev()
        .take_while(|index| line.char(*index).is_whitespace())
        .count();

    //debug_assert_eq!(leading + trimmed + trailing, line.len_chars());
    LinePortions {
        leading,
        trimmed,
        trailing,
    }
}

fn trimmed_range(
    buffer: &crate::buffer::Buffer,
    line_index: usize,
) -> anyhow::Result<Option<super::ByteRange>> {
    let Some(line) = buffer.get_line_by_line_index(line_index) else {
        return Ok(None);
    };

    let line_portions = get_line_portions(line);
    let line_start = buffer.line_to_char(line_index)?;
    let line_end = line_start + line.len_chars();

    let range = buffer.char_index_range_to_byte_range(
        (line_start + line_portions.leading..line_end - line_portions.trailing).into(),
    )?;
    Ok(Some(ByteRange::new(range)))
}

pub(crate) fn process_paste_gap(
    params: &SelectionModeParams,
    prev_gap: Option<String>,
    next_gap: Option<String>,
    direction: &Direction,
) -> String {
    let add_newline = |gap: String| {
        if gap.chars().any(|c| c == '\n') {
            gap
        } else {
            format!("\n{gap}")
        }
    };
    match (prev_gap, next_gap) {
        (None, None) => {
            // Get the indent of the current line
            let current_line = params
                .buffer
                .get_line_by_char_index(params.cursor_char_index())
                .unwrap_or_default();

            let indentation = current_line
                .chars()
                .take_while(|c| c.is_ascii_whitespace())
                .join("");

            add_newline(indentation)
        }
        (Some(gap), None) => add_newline(gap),
        (None, Some(gap)) => add_newline(gap),
        (Some(prev_gap), Some(next_gap)) => {
            let prev_gap = add_newline(prev_gap);
            let next_gap = add_newline(next_gap);
            let larger = next_gap.chars().count() > prev_gap.chars().count();
            match (direction, larger) {
                (Direction::Start, true) => prev_gap,
                (Direction::Start, false) => next_gap,
                (Direction::End, true) => next_gap,
                (Direction::End, false) => prev_gap,
            }
        }
    }
}

#[cfg(test)]
mod test_line {
    use crate::buffer::BufferOwner;
    use crate::components::editor::Movement;
    use crate::selection::SelectionMode;
    use crate::test_app::*;

    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    use serial_test::serial;

    use crate::selection_mode::{PositionBased, SelectionModeTrait};

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
                (26..26, ""),
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
                Editor(Paste),
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
    fn paste_backward_use_larger_indent() -> anyhow::Result<()> {
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
                Editor(SwapCursor),
                Editor(Paste),
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
    fn still_paste_forward_with_newline_with_indent_despite_only_one_line_present(
    ) -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("  foo".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(Copy),
                Editor(Paste),
                Expect(CurrentComponentContent("  \n  foo")),
            ])
        })
    }

    #[serial]
    #[test]
    fn still_paste_backward_with_newline_with_indent_despite_only_one_line_present(
    ) -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("  foo".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(Copy),
                Editor(SwapCursor),
                Editor(Paste),
                Expect(CurrentComponentContent("  \n  foo")),
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
                Editor(SwapCursor),
                Editor(Paste),
                Expect(CurrentComponentContent("foo\nbar\nbar")),
            ])
        })
    }

    #[serial]
    #[test]
    fn copy_pasting_backward_nothing_but_with_indentation() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(" ".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Expect(CurrentSelectedTexts(&[" "])),
                Editor(Copy),
                Editor(SwapCursor),
                Editor(Paste),
                Expect(CurrentComponentContent(" \n ")),
                Editor(Paste),
                Expect(CurrentComponentContent(" \n \n ")),
            ])
        })
    }

    #[serial]
    #[test]
    fn copy_pasting_forward_nothing_but_with_indentation() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(" ".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Expect(CurrentSelectedTexts(&[" "])),
                Editor(Copy),
                Editor(Paste),
                Expect(CurrentComponentContent(" \n ")),
                Editor(Paste),
                Expect(CurrentComponentContent(" \n \n ")),
            ])
        })
    }

    #[test]
    fn able_to_go_to_last_line_which_is_empty() -> anyhow::Result<()> {
        fn test(movement: Movement) -> anyhow::Result<()> {
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
                    Editor(MoveSelection(movement)),
                    Expect(EditorCursorPosition(crate::position::Position {
                        line: 1,
                        column: 0,
                    })),
                    Expect(CurrentSelectedTexts(&[""])),
                ])
            })
        }
        test(Movement::Last)?;
        test(Movement::Down)
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
                Expect(CurrentSelectedTexts(&[""])),
                Editor(Delete),
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
                Expect(CurrentSelectedTexts(&[""])),
                Editor(SwapCursor),
                Editor(Delete),
                Expect(CurrentComponentContent("hello")),
            ])
        })
    }
}
