use super::{ByteRange, PositionBasedSelectionMode, Word};
use crate::{buffer::Buffer, components::editor::IfCurrentNotFound, selection::CharIndex};

pub struct Subword;

const SUBWORD_REGEX: &str =
    r"[A-Z]{2,}(?=[A-Z][a-z])|[A-Z]{2,}|[A-Z][a-z]+|[A-Z]|[a-z]+|[^\w\s]|_|[0-9]+";

impl Subword {
    fn get_current_selection(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
        skip_symbols: bool,
    ) -> anyhow::Result<Option<ByteRange>> {
        let Some(last_char_index) = buffer.last_char_index() else {
            return Ok(None);
        };
        let cursor_char_index = cursor_char_index.min(last_char_index);

        let predicate = |c: char| {
            if skip_symbols {
                c.is_ascii_alphanumeric()
            } else {
                !c.is_whitespace()
            }
        };
        let current = {
            let mut current = cursor_char_index;
            loop {
                if (CharIndex(0)..=last_char_index).contains(&current) {
                    if predicate(buffer.char(current)?) {
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

        let current_char = buffer.char(current)?;

        if current_char.is_whitespace() || (skip_symbols && !current_char.is_ascii_alphanumeric()) {
            return Ok(None);
        }

        let range: crate::char_index_range::CharIndexRange = if current_char.is_ascii_lowercase() {
            let start = {
                let mut index = current;
                loop {
                    if index > CharIndex(0) && buffer.char(index - 1)?.is_ascii_lowercase() {
                        index = index - 1;
                    } else if index > CharIndex(0) && buffer.char(index - 1)?.is_ascii_uppercase() {
                        break index - 1;
                    } else {
                        break index;
                    }
                }
            };
            let end = {
                let mut index = current;
                loop {
                    if index < last_char_index && buffer.char(index + 1)?.is_ascii_lowercase() {
                        index = index + 1;
                    } else {
                        break index;
                    }
                }
            };
            start..end + 1
        } else if current_char.is_ascii_uppercase() {
            let start = {
                let mut index = current;
                if index < last_char_index && buffer.char(index + 1)?.is_lowercase() {
                    index
                } else {
                    loop {
                        if index == CharIndex(0) {
                            break index;
                        }
                        let char = buffer.char(index - 1)?;
                        if char.is_ascii_uppercase() {
                            index = index - 1;
                        } else {
                            break index;
                        }
                    }
                }
            };
            let end = {
                let mut previous_is_uppercase = buffer.char(current)?.is_ascii_uppercase();
                let mut index = current;
                loop {
                    if index >= last_char_index {
                        break index;
                    }
                    let char = buffer.char(index + 1)?;
                    if char.is_ascii_lowercase() {
                        previous_is_uppercase = char.is_ascii_uppercase();
                        index = index + 1;
                    } else if previous_is_uppercase && char.is_ascii_uppercase() {
                        if index < last_char_index - 1
                            && buffer.char(index + 2)?.is_ascii_lowercase()
                        {
                            break index;
                        } else {
                            previous_is_uppercase = char.is_ascii_uppercase();
                            index = index + 1;
                        }
                    } else {
                        break index;
                    }
                }
            };
            start..end + 1
        } else if current_char.is_ascii_digit() {
            let start = {
                let mut index = current;
                loop {
                    if index > CharIndex(0) && buffer.char(index - 1)?.is_ascii_digit() {
                        index = index - 1;
                    } else {
                        break index;
                    }
                }
            };
            let end = {
                let mut index = current;
                loop {
                    if index < last_char_index && buffer.char(index + 1)?.is_ascii_digit() {
                        index = index + 1;
                    } else {
                        break index;
                    }
                }
            };
            start..end + 1
        } else {
            current..current + 1
        }
        .into();

        Ok(Some(ByteRange::new(
            buffer.char_index_range_to_byte_range(range)?,
        )))
    }
}

impl PositionBasedSelectionMode for Subword {
    fn first(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_word(params, SelectionPosition::First)
    }

    fn last(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_word(params, SelectionPosition::Last)
    }

    fn get_current_meaningful_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        self.get_current_selection(buffer, cursor_char_index, if_current_not_found, true)
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<ByteRange>> {
        self.get_current_selection(buffer, cursor_char_index, if_current_not_found, false)
    }

    fn process_paste_gap(
        &self,
        _: &super::SelectionModeParams,
        prev_gap: Option<String>,
        next_gap: Option<String>,
        _: &crate::components::editor::Direction,
    ) -> String {
        super::word::process_paste_gap(prev_gap, next_gap)
    }
}

#[cfg(test)]
mod test_subword {
    use super::*;
    use crate::buffer::BufferOwner;
    use crate::components::editor::PriorChange;
    use crate::selection::SelectionMode;
    use crate::selection_mode::SelectionModeTrait as _;
    use crate::test_app::*;
    use crate::{buffer::Buffer, selection::Selection, selection_mode::PositionBased};

    use serial_test::serial;

    #[test]
    fn simple_case() {
        let buffer = Buffer::new(None, "snake Case camel");
        PositionBased(super::Subword).assert_all_selections(
            &buffer,
            Selection::default(),
            &[(0..5, "snake"), (6..10, "Case"), (11..16, "camel")],
        );
    }

    #[test]
    fn skip_symbols() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE ->() 123 <_> HTTPNetwork X",
        );
        PositionBased(super::Subword).assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..5, "snake"),
                (6..10, "case"),
                (11..16, "camel"),
                (16..20, "Case"),
                (21..27, "Pascal"),
                (27..31, "Case"),
                (32..37, "UPPER"),
                (38..43, "SNAKE"),
                (49..52, "123"),
                (57..61, "HTTP"),
                (61..68, "Network"),
                (69..70, "X"),
            ],
        );
    }

    #[test]
    fn no_skip_symbols() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("snake-case".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    crate::selection::SelectionMode::Subword,
                )),
                Editor(MoveSelectionWithPriorChange(
                    Next,
                    Some(PriorChange::EnterMultiCursorMode),
                )),
                Editor(MoveSelection(Next)),
                Expect(CurrentSelectedTexts(&["snake", "-", "case"])),
            ])
        })
    }

    #[test]
    fn consecutive_uppercase_letters() {
        let buffer = Buffer::new(None, "XMLParser JSONObject HTMLElement");
        PositionBased(super::Subword).assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..3, "XML"),
                (3..9, "Parser"),
                (10..14, "JSON"),
                (14..20, "Object"),
                (21..25, "HTML"),
                (25..32, "Element"),
            ],
        );
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
                    crate::selection::SelectionMode::Subword,
                )),
                // Expect selection mode not changed because there is zero possible selection
                Expect(CurrentSelectionMode(SelectionMode::Line)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&[""])),
                Editor(MoveSelection(Left)),
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
                Editor(SetContent("foo bar\nspam".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Subword,
                )),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(Copy),
                Editor(PasteWithMovement(Right)),
                Expect(CurrentComponentContent("foo bar bar\nspam")),
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
                Editor(SetContent("foo bar\nspam".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Subword,
                )),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&["bar"])),
                Editor(Copy),
                Editor(PasteWithMovement(Left)),
                Expect(CurrentComponentContent("foo bar bar\nspam")),
            ])
        })
    }
}

#[derive(Clone, Copy)]
pub enum SelectionPosition {
    First,
    Last,
}

fn get_word(
    params: &super::SelectionModeParams,
    position: SelectionPosition,
) -> anyhow::Result<Option<crate::selection::Selection>> {
    if let Some(current_word) = Word.current(
        params,
        crate::components::editor::IfCurrentNotFound::LookForward,
    )? {
        let content = params.buffer.slice(&current_word.range())?.to_string();
        let regex = fancy_regex::Regex::new(SUBWORD_REGEX)?;
        let mut captures = regex.captures_iter(&content);
        if let Some(match_) = match position {
            SelectionPosition::First => captures.next(),
            SelectionPosition::Last => captures.last(),
        } {
            let start = current_word.range().start;
            if let Some(range) = match_?.get(0).map(|m| start + m.start()..start + m.end()) {
                return Ok(Some(current_word.set_range(range.into())));
            }
        }
    }
    Ok(None)
}
