use ropey::Rope;

use crate::{components::editor::IfCurrentNotFound, selection::CharIndex};

use super::{ByteRange, PositionBasedSelectionMode};

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

impl PositionBasedSelectionMode for Word {
    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        get_current_word_by_cursor(true, buffer, cursor_char_index, if_current_not_found)
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        get_current_word_by_cursor(false, buffer, cursor_char_index, if_current_not_found)
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

fn is_word(char: char) -> bool {
    char.is_alphanumeric() || char == '_' || char == '-'
}

fn is_symbol(char: char) -> bool {
    !is_word(char) && !char.is_whitespace()
}

fn get_current_word_by_cursor(
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

#[cfg(test)]
mod test_word {
    use crate::buffer::BufferOwner;
    use crate::selection::SelectionMode;
    use crate::test_app::*;

    use super::*;

    use serial_test::serial;

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
                Editor(SetContent("foo ?bar:spam".to_string())),
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
                    prior_change: None,
                }),
                Expect(JumpChars(&['f', '?', 'b', ':', 's'])),
            ])
        })
    }

    #[test]
    fn gapless_delete_no_skip_symbols() -> anyhow::Result<()> {
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
                Editor(EnterDeleteMode),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["."])),
            ])
        })
    }

    #[test]
    fn delete_skip_symbols() -> anyhow::Result<()> {
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
                Editor(EnterDeleteMode),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Expect(CurrentComponentContent("bar.spam")),
            ])
        })
    }

    #[test]
    fn empty_buffer_should_not_be_word_selectable() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("".to_string())),
                Expect(CurrentSelectionMode(SelectionMode::Line)),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                // Expect selection mode not changed because there is zero possible selection
                Expect(CurrentSelectionMode(SelectionMode::Line)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Left)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&[""])),
            ])
        })
    }

    #[serial]
    #[test]
    fn paste_forward_with_gap() -> anyhow::Result<()> {
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
                Editor(Copy),
                Editor(Paste),
                Expect(CurrentComponentContent("fooFoo barBar barBar\nspamSpam")),
            ])
        })
    }

    #[serial]
    #[test]
    fn paste_backward_with_gap() -> anyhow::Result<()> {
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
                Editor(Copy),
                Editor(SwapCursor),
                Editor(Paste),
                Expect(CurrentComponentContent("fooFoo barBar barBar\nspamSpam")),
            ])
        })
    }

    #[test]
    fn next_previous_include_symbol() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo$bar#baz".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                Expect(CurrentSelectedTexts(&["foo"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["$"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["#"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["baz"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["#"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["$"])),
                Editor(MoveSelection(Previous)),
                Expect(CurrentSelectedTexts(&["foo"])),
            ])
        })
    }
}
