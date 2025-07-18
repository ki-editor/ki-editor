use ropey::Rope;

use crate::{components::editor::IfCurrentNotFound, selection::CharIndex};

use super::{ApplyMovementResult, ByteRange, PositionBasedSelectionMode, SelectionModeTrait};

pub struct Word;

fn find_word_start(rope: &Rope, current: CharIndex, is_word: impl Fn(char) -> bool) -> CharIndex {
    // Create a reverse range from current.0 down to 1 (not including 0)
    for i in (1..=current.0).rev() {
        let prev_char = rope.char(i - 1);
        if !is_word(prev_char) {
            return CharIndex(i);
        }
    }
    // If we've examined all characters to the start, return index 0
    CharIndex(0)
}

fn find_word_end(
    rope: &Rope,
    current: CharIndex,
    last_char_index: CharIndex,
    is_word: impl Fn(char) -> bool,
) -> CharIndex {
    // Create a range from current.0+1 to last_char_index.0
    for i in (current.0 + 1)..=last_char_index.0 {
        let char = rope.char(i);
        if !is_word(char) {
            return CharIndex(i - 1);
        }
    }
    // If we've examined all characters to the end, return the last index
    last_char_index
}

impl SelectionModeTrait for Word {
    fn left(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenSkipSymbol.left(params)
    }

    fn right(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenSkipSymbol.right(params)
    }

    fn delete_backward(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenNoSkipSymbol.left(params)
    }

    fn delete_forward(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenNoSkipSymbol.right(params)
    }

    fn current(
        &self,
        params: &super::SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenNoSkipSymbol.current(params, if_current_not_found)
    }

    #[cfg(test)]
    fn all_selections_gathered_inversely<'a>(
        &'a self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        TokenNoSkipSymbol.all_selections_gathered_inversely(params)
    }

    fn expand(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<super::ApplyMovementResult>> {
        params.expand()
    }

    fn up(
        &self,
        params: &super::SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        TokenNoSkipSymbol.up(params, sticky_column_index)
    }

    fn down(
        &self,
        params: &super::SelectionModeParams,
        sticky_column_index: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        TokenNoSkipSymbol.down(params, sticky_column_index)
    }

    fn selections_in_line_number_ranges(
        &self,
        params: &super::SelectionModeParams,
        line_number_ranges: Vec<std::ops::Range<usize>>,
    ) -> anyhow::Result<Vec<ByteRange>> {
        TokenNoSkipSymbol.selections_in_line_number_ranges(params, line_number_ranges)
    }

    fn to_index(
        &self,
        params: &super::SelectionModeParams,
        index: usize,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenNoSkipSymbol.to_index(params, index)
    }

    fn next(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenNoSkipSymbol.right(params)
    }

    fn previous(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenNoSkipSymbol.left(params)
    }

    fn process_paste_gap(
        &self,
        _: &super::SelectionModeParams,
        prev_gap: Option<String>,
        next_gap: Option<String>,
        _: &crate::components::editor::Direction,
    ) -> String {
        process_paste_gap(prev_gap, next_gap)
    }

    fn alpha(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(None)
    }

    fn omega(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(None)
    }

    fn first(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(None)
    }

    fn last(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(None)
    }
}

pub(crate) fn process_paste_gap(prev_gap: Option<String>, next_gap: Option<String>) -> String {
    match (prev_gap, next_gap) {
        (None, None) => Default::default(),
        (None, Some(gap)) | (Some(gap), None) => gap,
        (Some(prev_gap), Some(next_gap)) => {
            let trim = |s: String| {
                s.trim_end_matches('\n')
                    .trim_start_matches('\n')
                    .to_string()
            };
            let prev_gap = trim(prev_gap);
            let next_gap = trim(next_gap);
            if prev_gap.chars().count() > next_gap.chars().count() {
                prev_gap
            } else {
                next_gap
            }
        }
    }
}

struct TokenNoSkipSymbol;

impl PositionBasedSelectionMode for TokenNoSkipSymbol {
    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        get_current_token_by_cursor(false, buffer, cursor_char_index, if_current_not_found)
    }
}

struct TokenSkipSymbol;

impl PositionBasedSelectionMode for TokenSkipSymbol {
    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        get_current_token_by_cursor(true, buffer, cursor_char_index, if_current_not_found)
    }
}

fn is_word(char: char) -> bool {
    char.is_alphanumeric() || char == '_' || char == '-'
}

fn is_symbol(char: char) -> bool {
    !is_word(char) && !char.is_whitespace()
}

fn get_current_token_by_cursor(
    skip_symbols: bool,
    buffer: &crate::buffer::Buffer,
    cursor_char_index: crate::selection::CharIndex,
    if_current_not_found: IfCurrentNotFound,
) -> anyhow::Result<Option<super::ByteRange>> {
    let len_chars = buffer.len_chars();
    if len_chars == 0 {
        return Ok(None);
    }
    let last_char_index = CharIndex(len_chars - 1);

    // Define predicates once
    let is_target = |char: char| {
        if skip_symbols {
            is_word(char)
        } else {
            is_word(char) || is_symbol(char)
        }
    };

    if cursor_char_index > last_char_index {
        return Ok(None);
    }

    let last_char_index = CharIndex(buffer.len_chars().saturating_sub(1));

    let current = {
        let mut current = cursor_char_index;
        loop {
            if (CharIndex(0)..=last_char_index).contains(&current) {
                if is_target(buffer.char(current)?) {
                    break current;
                } else {
                    match if_current_not_found {
                        IfCurrentNotFound::LookForward if current < last_char_index => {
                            current = current + 1
                        }
                        IfCurrentNotFound::LookBackward if current > CharIndex(0) => {
                            current = current - 1
                        }
                        _ => break current,
                    }
                }
            } else {
                return Ok(None);
            }
        }
    };

    let rope = buffer.rope();
    if !is_target(rope.char(current.0)) {
        return Ok(None);
    }

    // Handle single symbol case
    if !skip_symbols && is_symbol(rope.char(current.0)) {
        let current_byte = rope.try_char_to_byte(current.0)?;
        return Ok(Some(ByteRange::new(current_byte..current_byte + 1)));
    }

    // Find word boundaries
    let start = find_word_start(rope, current, is_word);
    let end = find_word_end(rope, current, last_char_index, is_word) + 1;

    // Validate results
    debug_assert!(is_word(rope.char(current.0)));
    debug_assert!(is_word(rope.char(start.0)));
    debug_assert!(is_word(rope.char((end - 1).0)));

    Ok(Some(ByteRange::new(
        rope.try_char_to_byte(start.0)?..rope.try_char_to_byte(end.0)?,
    )))
}

#[cfg(test)]
mod test_word {
    use crate::buffer::BufferOwner;
    use crate::components::editor::Direction;
    use crate::selection::SelectionMode;
    use crate::test_app::*;

    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionModeTrait};

    use super::*;

    #[test]
    fn all_selections_no_skip_symbols() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE kebab-case ->() 123 <_>",
        );
        super::Word.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..10, "snake_case"),
                (11..20, "camelCase"),
                (21..31, "PascalCase"),
                (32..43, "UPPER_SNAKE"),
                (44..54, "kebab-case"),
                (55..56, "-"),
                (56..57, ">"),
                (57..58, "("),
                (58..59, ")"),
                (60..63, "123"),
                (64..65, "<"),
                (65..66, "_"),
                (66..67, ">"),
            ],
        );
    }

    #[test]
    fn alpha_beta_moves_to_symbols_only() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo bar ? spam : baz".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                Expect(CurrentSelectedTexts(&["foo"])),
                Editor(MoveSelection(Last)),
                Expect(CurrentSelectedTexts(&["?"])),
                Editor(MoveSelection(Last)),
                Expect(CurrentSelectedTexts(&[":"])),
                Editor(MoveSelection(First)),
                Expect(CurrentSelectedTexts(&["?"])),
            ])
        })
    }

    #[test]
    fn current_no_skip_symbols() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(".red".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                Expect(CurrentSelectedTexts(&["."])),
            ])
        })
    }

    #[test]
    fn up_down_no_skip_symbols() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(".foo\n=bar\n+spam".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                Expect(CurrentSelectedTexts(&["."])),
                Editor(MoveSelection(Down)),
                Expect(CurrentSelectedTexts(&["="])),
                Editor(MoveSelection(Down)),
                Expect(CurrentSelectedTexts(&["+"])),
                Editor(MoveSelection(Up)),
                Expect(CurrentSelectedTexts(&["="])),
                Editor(MoveSelection(Up)),
                Expect(CurrentSelectedTexts(&["."])),
            ])
        })
    }

    #[test]
    fn jump_no_skip_symbols() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo ? bar : spam".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                App(TerminalDimensionChanged(crate::app::Dimension {
                    height: 3,
                    width: 50,
                })),
                Editor(ShowJumps {
                    use_current_selection_mode: true,
                }),
                Expect(JumpChars(&['f', '?', 'b', ':', 's'])),
            ])
        })
    }

    #[test]
    fn delete_no_skip_symbols() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo.bar.spam".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                Expect(CurrentSelectedTexts(&["foo"])),
                Editor(Delete(Direction::End)),
                Expect(CurrentSelectedTexts(&["."])),
            ])
        })
    }

    #[test]
    fn empty_buffer_should_not_be_token_selectable() -> anyhow::Result<()> {
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
                    SelectionMode::Word,
                )),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Down)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Up)),
                Expect(CurrentSelectedTexts(&[""])),
            ])
        })
    }

    #[test]
    fn paste_gap() -> anyhow::Result<()> {
        let run_test = |direction: Direction| {
            execute_test(|s| {
                Box::new([
                    App(OpenFile {
                        path: s.main_rs(),
                        owner: BufferOwner::User,
                        focus: true,
                    }),
                    Editor(SetContent("fooFoo barBar\nspamSpam".to_string())),
                    Editor(SetSelectionMode(
                        IfCurrentNotFound::LookForward,
                        SelectionMode::Word,
                    )),
                    Editor(MoveSelection(Right)),
                    Expect(CurrentSelectedTexts(&["barBar"])),
                    Editor(Copy {
                        use_system_clipboard: false,
                    }),
                    Editor(Paste {
                        use_system_clipboard: false,
                        direction: direction.clone(),
                    }),
                    Expect(CurrentComponentContent("fooFoo barBar barBar\nspamSpam")),
                ])
            })
        };
        run_test(Direction::End)?;
        run_test(Direction::Start)
    }
}
