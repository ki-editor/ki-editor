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
        let start_index;
        let end_index;

        if cursor_char_index == CharIndex(0) {
            let mut right_index = cursor_char_index;
            let mut right_atleast_one_non_whitespace = false;
            let mut right_first_non_whitespace = CharIndex(0);
            let mut right_last_non_whitespace = CharIndex(0);
            loop {
                if right_index == CharIndex(buffer.len_chars()) {
                    break;
                }
                let Ok(ch) = buffer.char(right_index) else {
                    break;
                };
                if ch == '\n' {
                    break;
                } else if !ch.is_whitespace() {
                    if !right_atleast_one_non_whitespace {
                        right_first_non_whitespace = right_index;
                    }
                    right_last_non_whitespace = right_index;
                    right_atleast_one_non_whitespace = true;
                }
                right_index = right_index + 1;
            }
            if right_atleast_one_non_whitespace {
                start_index = right_first_non_whitespace;
                end_index = right_last_non_whitespace + 1;
            } else {
                start_index = right_index;
                end_index = right_index;
            }
        } else if cursor_char_index == CharIndex(buffer.len_chars()) {
            match if_current_not_found {
                IfCurrentNotFound::LookForward => {
                    start_index = CharIndex(buffer.len_chars());
                    end_index = CharIndex(buffer.len_chars());
                }
                IfCurrentNotFound::LookBackward => {
                    let mut left_index = cursor_char_index;
                    let mut left_atleast_one_non_whitespace = false;
                    let mut left_first_non_whitespace = CharIndex(0);
                    let mut left_last_non_whitespace = CharIndex(0);
                    loop {
                        if left_index == CharIndex(0) {
                            break;
                        };
                        left_index = left_index - 1;
                        let Ok(ch) = buffer.char(left_index) else {
                            break;
                        };
                        if ch == '\n' {
                            break;
                        } else if ch.is_whitespace() {
                            continue;
                        } else {
                            if !left_atleast_one_non_whitespace {
                                left_first_non_whitespace = left_index;
                            }
                            left_atleast_one_non_whitespace = true;
                            left_last_non_whitespace = left_index;
                        }
                    }
                    if left_atleast_one_non_whitespace {
                        start_index = left_last_non_whitespace;
                        end_index = left_first_non_whitespace + 1;
                    } else {
                        start_index = CharIndex(buffer.len_chars());
                        end_index = CharIndex(buffer.len_chars());
                    }
                }
            }
        } else {
            let Ok(ch) = buffer.char(cursor_char_index) else {
                return Ok(None);
            };

            if ch == '\n' {
                match if_current_not_found {
                    IfCurrentNotFound::LookForward => {
                        let mut right_atleast_one_non_whitespace = false;
                        let mut right_first_non_whitespace = CharIndex(0);
                        let mut right_last_non_whitespace = CharIndex(0);

                        let mut right_index = cursor_char_index;
                        loop {
                            right_index = right_index + 1;

                            let Ok(ch) = buffer.char(right_index) else {
                                break;
                            };

                            if ch == '\n' {
                                break;
                            } else if ch.is_whitespace() {
                                continue;
                            } else {
                                if !right_atleast_one_non_whitespace {
                                    right_first_non_whitespace = right_index;
                                }
                                right_atleast_one_non_whitespace = true;
                                right_last_non_whitespace = right_index;
                            }
                        }

                        if right_atleast_one_non_whitespace {
                            start_index = right_first_non_whitespace;
                            end_index = right_last_non_whitespace + 1;
                        } else {
                            start_index = right_index;
                            end_index = right_index;
                        }
                    }
                    IfCurrentNotFound::LookBackward => {
                        let mut left_outer_index = cursor_char_index;
                        let mut left_atleast_one_non_whitespace = false;
                        let mut left_first_non_whitespace = CharIndex(0);
                        let mut left_last_non_whitespace = CharIndex(0);

                        loop {
                            if left_outer_index == CharIndex(0) {
                                break;
                            }

                            left_outer_index = left_outer_index - 1;

                            let Ok(ch) = buffer.char(left_outer_index) else {
                                break;
                            };

                            if ch == '\n' {
                                break;
                            } else if ch.is_whitespace() {
                                continue;
                            } else {
                                if !left_atleast_one_non_whitespace {
                                    left_first_non_whitespace = left_outer_index;
                                }
                                left_atleast_one_non_whitespace = true;
                                left_last_non_whitespace = left_outer_index;
                            }
                        }

                        if left_atleast_one_non_whitespace {
                            start_index = left_last_non_whitespace;
                            end_index = left_first_non_whitespace + 1;
                        } else {
                            start_index = cursor_char_index;
                            end_index = cursor_char_index;
                        }
                    }
                }
            } else if ch.is_whitespace() {
                let mut left_index = cursor_char_index;
                let mut right_index = cursor_char_index;

                let mut left_atleast_one_non_whitespace = false;

                loop {
                    if left_index == CharIndex(0) {
                        break;
                    }
                    left_index = left_index - 1;

                    let Ok(ch) = buffer.char(left_index) else {
                        break;
                    };

                    if ch == '\n' {
                        break;
                    } else if ch.is_whitespace() {
                        continue;
                    } else {
                        left_atleast_one_non_whitespace = true;
                    }
                }

                let mut right_atleast_one_non_whitespace = false;
                let mut right_first_non_whitespace = CharIndex(0);
                let mut right_last_non_whitespace = CharIndex(0);

                loop {
                    if right_index == CharIndex(buffer.len_chars()) {
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
                        if !right_atleast_one_non_whitespace {
                            right_first_non_whitespace = right_index
                        }
                        right_atleast_one_non_whitespace = true;
                        right_last_non_whitespace = right_index;
                    }
                }

                match if_current_not_found {
                    IfCurrentNotFound::LookForward => {
                        if left_atleast_one_non_whitespace {
                            let mut right_inner_index = right_index;
                            let mut right_atleast_one_non_whitespace = false;
                            let mut right_first_non_whitespace = CharIndex(0);
                            let mut right_last_non_whitespace = CharIndex(0);
                            loop {
                                if right_inner_index == CharIndex(buffer.len_chars()) {
                                    break;
                                }
                                right_inner_index = right_inner_index + 1;
                                let Ok(ch) = buffer.char(right_inner_index) else {
                                    break;
                                };
                                if ch == '\n' {
                                    break;
                                } else if ch.is_whitespace() {
                                    continue;
                                } else {
                                    if !right_atleast_one_non_whitespace {
                                        right_first_non_whitespace = right_inner_index;
                                    }
                                    right_atleast_one_non_whitespace = true;
                                    right_last_non_whitespace = right_inner_index;
                                }
                            }
                            if right_atleast_one_non_whitespace {
                                start_index = right_first_non_whitespace;
                                end_index = right_last_non_whitespace + 1;
                            } else {
                                start_index = right_inner_index;
                                end_index = right_inner_index;
                            }
                        } else if right_atleast_one_non_whitespace {
                            start_index = right_first_non_whitespace;
                            end_index = right_last_non_whitespace + 1;
                        } else {
                            start_index = right_index;
                            end_index = right_index;
                        }
                    }
                    IfCurrentNotFound::LookBackward => {
                        if left_atleast_one_non_whitespace {
                            let mut right_inner_index = right_index;
                            let mut right_atleast_one_non_whitespace = false;
                            let mut right_first_non_whitespace = CharIndex(0);
                            let mut right_last_non_whitespace = CharIndex(0);
                            loop {
                                if right_inner_index == CharIndex(buffer.len_chars()) {
                                    break;
                                }
                                right_inner_index = right_inner_index + 1;
                                let Ok(ch) = buffer.char(right_inner_index) else {
                                    break;
                                };
                                if ch == '\n' {
                                    break;
                                } else if ch.is_whitespace() {
                                    continue;
                                } else {
                                    if !right_atleast_one_non_whitespace {
                                        right_first_non_whitespace = right_inner_index;
                                    }
                                    right_atleast_one_non_whitespace = true;
                                    right_last_non_whitespace = right_inner_index;
                                }
                            }
                            if right_atleast_one_non_whitespace {
                                start_index = right_first_non_whitespace;
                                end_index = right_last_non_whitespace + 1;
                            } else {
                                start_index = right_inner_index;
                                end_index = right_inner_index;
                            }
                        } else {
                            let mut left_index_inner = left_index;
                            let mut left_atleast_one_non_whitespace = false;
                            let mut left_first_non_whitespace_inner = CharIndex(0);
                            let mut left_last_non_whitespace_inner = CharIndex(0);
                            loop {
                                if left_index_inner == CharIndex(0) {
                                    break;
                                }
                                left_index_inner = left_index_inner - 1;
                                let Ok(ch) = buffer.char(left_index_inner) else {
                                    break;
                                };
                                if ch == '\n' {
                                    break;
                                } else if ch.is_whitespace() {
                                    continue;
                                } else {
                                    if !left_atleast_one_non_whitespace {
                                        left_first_non_whitespace_inner = left_index_inner;
                                    }
                                    left_atleast_one_non_whitespace = true;
                                    left_last_non_whitespace_inner = left_index_inner;
                                }
                            }
                            if left_atleast_one_non_whitespace {
                                start_index = left_last_non_whitespace_inner;
                                end_index = left_first_non_whitespace_inner + 1;
                            } else {
                                start_index = left_index;
                                end_index = left_index;
                            }
                        }
                    }
                }
            } else {
                let mut right_index = cursor_char_index;
                let mut right_atleast_one_non_whitespace = false;
                let mut right_last_non_whitespace = CharIndex(0);
                loop {
                    if right_index == CharIndex(buffer.len_chars()) {
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
                        right_atleast_one_non_whitespace = true;
                        right_last_non_whitespace = right_index;
                    }
                }
                let mut left_index = cursor_char_index;
                let mut left_atleast_one_non_whitespace = false;
                let mut left_last_non_whitespace = CharIndex(0);
                loop {
                    if left_index == CharIndex(0) {
                        break;
                    }
                    left_index = left_index - 1;
                    let Ok(ch) = buffer.char(left_index) else {
                        break;
                    };
                    if ch == '\n' {
                        break;
                    } else if ch.is_whitespace() {
                        continue;
                    } else {
                        left_atleast_one_non_whitespace = true;
                        left_last_non_whitespace = left_index;
                    }
                }
                if left_atleast_one_non_whitespace {
                    start_index = left_last_non_whitespace;
                } else {
                    start_index = cursor_char_index;
                }
                if right_atleast_one_non_whitespace {
                    end_index = right_last_non_whitespace + 1;
                } else {
                    end_index = cursor_char_index + 1;
                }
            };
        }

        let trimmed_range =
            buffer.char_index_range_to_byte_range((start_index..end_index).into())?;
        Ok(Some(ByteRange::new(trimmed_range)))
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        _: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let start_index;
        let end_index;

        if cursor_char_index == CharIndex(buffer.len_chars()) {
            start_index = cursor_char_index;
            end_index = cursor_char_index;
        } else {
            let Ok(ch) = buffer.char(cursor_char_index) else {
                return Ok(None);
            };

            if cursor_char_index == CharIndex(buffer.len_chars()) {
                start_index = cursor_char_index;
                end_index = cursor_char_index;
            } else if ch == '\n' {
                let mut left_atleast_one_non_whitespace = false;
                let mut left_first_non_whitespace = CharIndex(0);

                let mut left_index = cursor_char_index;
                loop {
                    if left_index.0 == 0 {
                        break;
                    }

                    left_index = left_index - 1;
                    let Ok(ch) = buffer.char(left_index) else {
                        break;
                    };
                    if ch == '\n' {
                        break;
                    } else if ch.is_whitespace() {
                        continue;
                    } else {
                        left_atleast_one_non_whitespace = true;
                        left_first_non_whitespace = left_index;
                        break;
                    }
                }

                if left_atleast_one_non_whitespace {
                    start_index = left_first_non_whitespace + 1;
                    end_index = cursor_char_index + 1;
                } else {
                    start_index = cursor_char_index;
                    end_index = cursor_char_index + 1;
                }
            } else if ch.is_whitespace() {
                {
                    let mut left_index = cursor_char_index;
                    let mut left_atleast_one_non_whitespace = false;
                    let mut left_last_non_whitespace = CharIndex(0);
                    let mut left_first_non_whitespace = CharIndex(0);
                    loop {
                        if left_index == CharIndex(0) {
                            break;
                        }

                        left_index = left_index - 1;
                        let Ok(ch) = buffer.char(left_index) else {
                            break;
                        };
                        if ch == '\n' {
                            left_index = left_index + 1;
                            break;
                        } else if ch.is_whitespace() {
                            continue;
                        } else {
                            if !left_atleast_one_non_whitespace {
                                left_first_non_whitespace = left_index;
                            }
                            left_atleast_one_non_whitespace = true;
                            left_last_non_whitespace = left_index;
                        }
                    }

                    let mut right_index = cursor_char_index;
                    let mut right_atleast_one_non_whitespace = false;
                    let mut right_last_non_whitespace = CharIndex(0);
                    let mut right_first_non_whitespace = CharIndex(0);
                    loop {
                        if right_index == CharIndex(buffer.len_chars()) {
                            break;
                        }

                        right_index = right_index + 1;
                        let Ok(ch) = buffer.char(right_index) else {
                            break;
                        };
                        if ch == '\n' {
                            right_index = right_index + 1;
                            break;
                        } else if ch.is_whitespace() {
                            continue;
                        } else {
                            if !right_atleast_one_non_whitespace {
                                right_first_non_whitespace = right_index;
                            }
                            right_atleast_one_non_whitespace = true;
                            right_last_non_whitespace = right_index;
                        }
                    }

                    if left_atleast_one_non_whitespace {
                        if right_atleast_one_non_whitespace {
                            start_index = left_last_non_whitespace;
                            end_index = right_last_non_whitespace + 1;
                        } else {
                            start_index = left_first_non_whitespace + 1;
                            end_index = right_index;
                        }
                    } else {
                        start_index = left_index;
                        if right_atleast_one_non_whitespace {
                            end_index = right_first_non_whitespace;
                        } else {
                            end_index = right_index;
                        }
                    }
                }
            } else {
                let mut left_index = cursor_char_index;

                let mut left_most_non_whitespace = cursor_char_index;
                start_index = loop {
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

                end_index = {
                    let mut right_encountered_non_whitespace = false;
                    let mut right_last_non_whitespace = CharIndex(0);
                    let mut right_index = left_most_non_whitespace;
                    loop {
                        if right_index.0 == buffer.len_chars() {
                            if right_encountered_non_whitespace {
                                break right_last_non_whitespace + 1;
                            } else {
                                break CharIndex(buffer.len_chars());
                            }
                        }

                        right_index = right_index + 1;
                        let Ok(ch) = buffer.char(right_index) else {
                            if right_encountered_non_whitespace {
                                break right_last_non_whitespace + 1;
                            } else {
                                break right_index;
                            }
                        };
                        if ch == '\n' {
                            if right_encountered_non_whitespace {
                                break right_last_non_whitespace + 1;
                            } else {
                                break right_index;
                            }
                        } else if ch.is_whitespace() {
                            continue;
                        } else {
                            right_encountered_non_whitespace = true;
                            right_last_non_whitespace = right_index;
                        }
                    }
                }
            }
        }
        let trimmed_range =
            buffer.char_index_range_to_byte_range((start_index..end_index).into())?;
        Ok(Some(ByteRange::new(trimmed_range)))
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
    fn left_right_movement() {
        let buffer = Buffer::new(None, "a\n\nb");
        PositionBased(LineTrimmed).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..1, "a"), (2..2, ""), (3..4, "b")],
        );
    }

    #[test]
    fn prev_next_movement() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("   a   \n    \nb".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Expect(CurrentSelectedTexts(&["   "])),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["a"])),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["   \n"])),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["    "])),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["\n"])),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["b"])),
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
🦀  main.rs [*]
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
                Expect(CurrentSelectedTexts(&["🦀  main.rs [*]"])),
                Editor(MoveSelection(Movement::Right)),
                Expect(CurrentSelectedTexts(&["1│fn first () {"])),
                Editor(MoveSelection(Movement::Right)),
                Expect(CurrentSelectedTexts(&["5│  █ifth();"])),
                Editor(MoveSelection(Movement::Left)),
                Expect(CurrentSelectedTexts(&["1│fn first () {"])),
            ])
        })
    }
}
