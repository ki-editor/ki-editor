use ropey::Rope;

use crate::{components::editor::IfCurrentNotFound, selection::CharIndex};

use super::{ApplyMovementResult, ByteRange, PositionBasedSelectionMode, SelectionModeTrait};

pub struct Token;

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

fn find_whitespace_start(rope: &Rope, current: CharIndex) -> CharIndex {
    // Create a reverse range from current.0 down to 1 (not including 0)
    for i in (1..=current.0).rev() {
        let prev_char = rope.char(i - 1);
        if !prev_char.is_whitespace() {
            return CharIndex(i);
        }
    }
    // If we've examined all characters to the start, return index 0
    CharIndex(0)
}

fn find_whitespace_end(rope: &Rope, current: CharIndex, last_char_index: CharIndex) -> CharIndex {
    // Create a range from current.0+1 to last_char_index.0
    for i in (current.0 + 1)..=last_char_index.0 {
        let char = rope.char(i);
        if !char.is_whitespace() {
            return CharIndex(i - 1);
        }
    }
    // If we've examined all characters to the end, return the last index
    last_char_index
}

impl SelectionModeTrait for Token {
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
        TokenIncludeWhitespace.all_selections_gathered_inversely(params)
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
        TokenIncludeWhitespace.to_index(params, index)
    }

    fn next(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenIncludeWhitespace.right(params)
    }

    fn previous(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TokenIncludeWhitespace.left(params)
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

    fn first(
        &self,
        _: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        Ok(None)
    }

    fn last(
        &self,
        _: &super::SelectionModeParams,
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

struct TokenIncludeWhitespace;

impl PositionBasedSelectionMode for TokenIncludeWhitespace {
    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        get_current_token_or_whitespace_by_cursor(buffer, cursor_char_index, if_current_not_found)
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
    let Some(last_char_index) = buffer.last_char_index() else {
        return Ok(None);
    };

    let is_target = |char: char| {
        if skip_symbols {
            is_word(char)
        } else {
            is_word(char) || is_symbol(char)
        }
    };

    let cursor_char_index = cursor_char_index.min(last_char_index);

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

fn get_current_token_or_whitespace_by_cursor(
    buffer: &crate::buffer::Buffer,
    cursor_char_index: crate::selection::CharIndex,
    if_current_not_found: IfCurrentNotFound,
) -> anyhow::Result<Option<super::ByteRange>> {
    let Some(last_char_index) = buffer.last_char_index() else {
        return Ok(None);
    };

    let cursor_char_index = cursor_char_index.min(last_char_index);
    let rope = buffer.rope();

    let is_target = |char: char| is_word(char) || is_symbol(char) || char.is_whitespace();

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

    let current_char = rope.char(current.0);

    if !is_target(current_char) {
        return Ok(None);
    }

    // Handle whitespace
    if current_char.is_whitespace() {
        let start = find_whitespace_start(rope, current);
        let end = find_whitespace_end(rope, current, last_char_index) + 1;

        return Ok(Some(ByteRange::new(
            rope.try_char_to_byte(start.0)?..rope.try_char_to_byte(end.0)?,
        )));
    }

    // Handle single symbol case
    if is_symbol(current_char) {
        let current_byte = rope.try_char_to_byte(current.0)?;
        return Ok(Some(ByteRange::new(current_byte..current_byte + 1)));
    }

    // Handle words
    let start = find_word_start(rope, current, is_word);
    let end = find_word_end(rope, current, last_char_index, is_word) + 1;

    Ok(Some(ByteRange::new(
        rope.try_char_to_byte(start.0)?..rope.try_char_to_byte(end.0)?,
    )))
}

#[cfg(test)]
mod test_token {
    use crate::buffer::BufferOwner;
    use crate::components::editor::Direction;
    use crate::selection::SelectionMode;
    use crate::test_app::*;

    use super::*;

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
                    SelectionMode::Token,
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
                    SelectionMode::Token,
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
                    SelectionMode::Token,
                )),
                App(TerminalDimensionChanged(crate::app::Dimension {
                    height: 3,
                    width: 50,
                })),
                Editor(ShowJumps {
                    use_current_selection_mode: true,
                    prior_change: None,
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
                    SelectionMode::Token,
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
                    SelectionMode::Token,
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
                        SelectionMode::Token,
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

    #[test]
    fn next_previous_include_whitespace() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo  bar   baz\nspam".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Token,
                )),
                Expect(CurrentSelectedTexts(&["foo"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["  "])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["   "])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["baz"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["\n"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["spam"])),
                Editor(MoveSelection(Previous)),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["baz"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["   "])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["  "])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["foo"])),
            ])
        })
    }
}
