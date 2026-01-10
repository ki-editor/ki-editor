use ropey::Rope;

use crate::{components::editor::IfCurrentNotFound, selection::CharIndex};

use super::{ByteRange, PositionBasedSelectionMode};

pub struct BigWord;

fn find_word_start(rope: &Rope, current: CharIndex) -> CharIndex {
    // Create a reverse range from current.0 down to 1 (not including 0)
    for i in (1..=current.0).rev() {
        let prev_char = rope.char(i - 1);
        if prev_char.is_whitespace() {
            return CharIndex(i);
        }
    }
    // If we've examined all characters to the start, return index 0
    CharIndex(0)
}

fn find_word_end(rope: &Rope, current: CharIndex, last_char_index: CharIndex) -> CharIndex {
    // Create a range from current.0+1 to last_char_index.0
    for i in (current.0 + 1)..=last_char_index.0 {
        let next_char = rope.char(i);
        if next_char.is_whitespace() {
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
        if !prev_char.is_whitespace() || prev_char == '\n' {
            return CharIndex(i);
        }
    }
    // If we've examined all characters to the start, return index 0
    CharIndex(0)
}

fn find_whitespace_end(rope: &Rope, current: CharIndex, last_char_index: CharIndex) -> CharIndex {
    // Create a range from current.0+1 to last_char_index.0
    for i in (current.0 + 1)..=last_char_index.0 {
        let next_char = rope.char(i);
        if !next_char.is_whitespace() || next_char == '\n' {
            return CharIndex(i - 1);
        }
    }
    // If we've examined all characters to the end, return the last index
    last_char_index
}

impl PositionBasedSelectionMode for BigWord {
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

pub fn process_paste_gap(prev_gap: Option<String>, next_gap: Option<String>) -> String {
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

fn get_current_word_by_cursor(
    skip_whitespaces: bool,
    buffer: &crate::buffer::Buffer,
    cursor_char_index: crate::selection::CharIndex,
    if_current_not_found: IfCurrentNotFound,
) -> anyhow::Result<Option<super::ByteRange>> {
    let Some(last_char_index) = buffer.last_char_index() else {
        return Ok(None);
    };

    let is_target = |char: char| {
        if skip_whitespaces {
            !char.is_whitespace()
        } else {
            // Any char is a target
            true
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
    let current_char = rope.char(current.0);

    if !is_target(current_char) {
        return Ok(None);
    }

    let (start, end) = if current_char == '\n' {
        (current, current + 1)
    } else if !skip_whitespaces && current_char.is_whitespace() {
        // Find whitespace boundaries
        let start = find_whitespace_start(rope, current);
        let end = find_whitespace_end(rope, current, last_char_index) + 1;

        (start, end)
    } else {
        // Find word boundaries
        let start = find_word_start(rope, current);
        let end = find_word_end(rope, current, last_char_index) + 1;

        (start, end)
    };
    Ok(Some(ByteRange::new(
        rope.try_char_to_byte(start.0)?..rope.try_char_to_byte(end.0)?,
    )))
}

#[cfg(test)]
mod test_word {
    use crate::buffer::BufferOwner;

    use crate::components::editor::Movement;
    use crate::selection::SelectionMode;
    use crate::test_app::*;

    use super::*;

    #[test]
    fn all_selections_no_skip_whitespaces() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello/foo    bar".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::BigWord,
                )),
                Expect(CurrentSelectedTexts(&["hello/foo"])),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["    "])),
                Editor(MoveSelection(Movement::Next)),
                Expect(CurrentSelectedTexts(&["bar"])),
            ])
        })
    }

    #[test]
    fn meaningful_selections_skip_whitespaces() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("hello/foo    bar".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::BigWord,
                )),
                Expect(CurrentSelectedTexts(&["hello/foo"])),
                Editor(MoveSelection(Movement::Right)),
                Expect(CurrentSelectedTexts(&["bar"])),
            ])
        })
    }

    #[test]
    fn newline_is_its_own_selection() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("\n    bar".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::BigWord,
                )),
                Expect(CurrentSelectedTexts(&["\n"])),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["    "])),
            ])
        })
    }
}
