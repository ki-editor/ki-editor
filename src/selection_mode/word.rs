use super::{ByteRange, PositionBasedSelectionMode, SelectionModeTrait, Token};
use crate::{buffer::Buffer, components::editor::IfCurrentNotFound, selection::CharIndex};

pub struct Word {
    skip_symbols: bool,
}

const SUBWORD_REGEX: &str =
    r"[A-Z]{2,}(?=[A-Z][a-z])|[A-Z]{2,}|[A-Z][a-z]+|[A-Z]|[a-z]+|[^\w\s]|_|[0-9]+";

impl Word {
    pub(crate) fn new(skip_symbols: bool) -> Self {
        Self { skip_symbols }
    }

    fn get_current_selection(
        &self,
        buffer: &Buffer,
        cursor_char_index: CharIndex,
        if_current_not_found: IfCurrentNotFound,
        skip_symbols: bool,
    ) -> anyhow::Result<Option<ByteRange>> {
        let rope = buffer.rope();
        let len_chars = rope.len_chars();
        if len_chars == 0 {
            return Ok(None);
        }
        let last_char_index = CharIndex(len_chars - 1);

        if cursor_char_index > last_char_index {
            return Ok(None);
        }

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

impl PositionBasedSelectionMode for Word {
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
        super::token::process_paste_gap(prev_gap, next_gap)
    }
}

#[cfg(test)]
mod test_word {
    use super::*;
    use crate::buffer::BufferOwner;
    use crate::components::editor::Direction;
    use crate::selection::SelectionMode;
    use crate::test_app::*;
    use crate::{buffer::Buffer, selection::Selection, selection_mode::PositionBased};

    #[test]
    fn simple_case() {
        let buffer = Buffer::new(None, "snake Case camel");
        PositionBased(super::Word::new(true)).assert_all_selections(
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
        PositionBased(super::Word::new(true)).assert_all_selections(
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
    fn no_skip_symbols() {
        let buffer = Buffer::new(
            None,
            "snake_case camelCase PascalCase UPPER_SNAKE ->() 123 <_> HTTPNetwork X",
        );
        PositionBased(super::Word::new(false)).assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..5, "snake"),
                (5..6, "_"),
                (6..10, "case"),
                (11..16, "camel"),
                (16..20, "Case"),
                (21..27, "Pascal"),
                (27..31, "Case"),
                (32..37, "UPPER"),
                (37..38, "_"),
                (38..43, "SNAKE"),
                (44..45, "-"),
                (45..46, ">"),
                (46..47, "("),
                (47..48, ")"),
                (49..52, "123"),
                (53..54, "<"),
                (54..55, "_"),
                (55..56, ">"),
                (57..61, "HTTP"),
                (61..68, "Network"),
                (69..70, "X"),
            ],
        );
    }

    #[test]
    fn consecutive_uppercase_letters() {
        let buffer = Buffer::new(None, "XMLParser JSONObject HTMLElement");
        PositionBased(super::Word::new(true)).assert_all_selections(
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
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    crate::selection::SelectionMode::Word {
                        skip_symbols: false,
                    },
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
                    Editor(SetContent("foo bar\nspam".to_string())),
                    Editor(SetSelectionMode(
                        IfCurrentNotFound::LookForward,
                        SelectionMode::Word {
                            skip_symbols: false,
                        },
                    )),
                    Editor(MoveSelection(Right)),
                    Expect(CurrentSelectedTexts(&["bar"])),
                    Editor(Copy {
                        use_system_clipboard: false,
                    }),
                    Editor(Paste {
                        use_system_clipboard: false,
                        direction: direction.clone(),
                    }),
                    Expect(CurrentComponentContent("foo bar bar\nspam")),
                ])
            })
        };
        run_test(Direction::End)?;
        run_test(Direction::Start)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SelectionPosition {
    First,
    Last,
}

fn get_word(
    params: &super::SelectionModeParams,
    position: SelectionPosition,
) -> anyhow::Result<Option<crate::selection::Selection>> {
    if let Some(current_word) = Token.current(
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
