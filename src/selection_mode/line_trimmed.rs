use itertools::Itertools;

use crate::{
    components::editor::{Direction, IfCurrentNotFound},
    selection::{CharIndex, Selection},
};

use super::{
    ByteRange, PositionBased, PositionBasedSelectionMode, SelectionModeParams, SelectionModeTrait,
};

#[derive(Clone)]
pub(crate) struct LineTrimmed;

impl PositionBasedSelectionMode for LineTrimmed {
    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        _: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let max_cursor_char_index = CharIndex(buffer.len_chars());
        if cursor_char_index > max_cursor_char_index {
            return Ok(None);
        }

        let line_index = buffer.char_to_line(cursor_char_index)?;
        let Some(line) = buffer.get_line_by_line_index(line_index) else {
            return Ok(None);
        };
        let line_start_char_index = buffer.line_to_char(line_index)?;
        let leading_whitespace_count = line
            .chars()
            .take_while(|c| c.is_whitespace() && c != &'\n')
            .count();
        if line.chars().all(|c| c.is_whitespace()) {
            let line_start_byte_index =
                buffer.char_to_byte(line_start_char_index + leading_whitespace_count - 0)?;
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
            ..
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

    fn right(&self, params: &SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        Ok(self.down(params, None)?.map(|result| result.selection))
    }

    fn next(
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
                    return self.to_index(params, line_index);
                } else {
                    line_index += 1
                }
            } else {
                break;
            }
        }
        Ok(None)
    }

    fn previous(
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
                return self.to_index(params, line_index);
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
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(PositionBased(self.clone())
            .down(params, None)?
            .map(|result| result.selection))
    }

    fn delete_backward(
        &self,
        params: &SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(PositionBased(self.clone())
            .up(params, None)?
            .map(|result| result.selection))
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
    use crate::buffer::BufferOwner;
    use crate::components::editor::Movement;
    use crate::selection::SelectionMode;
    use crate::test_app::*;

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
                Expect(CurrentComponentContent("  foo\n  foo")),
            ])
        })
    }

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
                Expect(CurrentComponentContent("  foo\n  foo")),
            ])
        })
    }

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
                Expect(CurrentSelectedTexts(&[""])),
                Editor(Copy),
                Editor(SwapCursor),
                Editor(Paste),
                Expect(CurrentComponentContent(" \n ")),
                Editor(Paste),
                Expect(CurrentComponentContent(" \n \n ")),
            ])
        })
    }

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
                Expect(CurrentSelectedTexts(&[""])),
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
