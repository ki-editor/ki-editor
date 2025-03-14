use ropey::Rope;

use crate::selection::Selection;

use super::{word::SelectionPosition, ByteRange, SelectionMode, SelectionModeParams, Word};

pub(crate) struct Character {
    current_column: usize,
}

impl Character {
    pub(crate) fn new(current_column: usize) -> Self {
        Self { current_column }
    }
}

impl SelectionMode for Character {
    fn iter<'a>(
        &'a self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
            ..
        }: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let char_index = current_selection.to_char_index(cursor_direction);
        let line = buffer.char_to_line(char_index)?;
        let line_start_char_index = buffer.line_to_char(line)?;
        let current_line = buffer.get_line_by_char_index(char_index)?;
        let line_len = line_len_without_new_line(&current_line);

        Ok(Box::new((0..line_len).flat_map(
            move |column| -> anyhow::Result<ByteRange> {
                let byte = buffer.char_to_byte(line_start_char_index + column)?;
                Ok(ByteRange::new(byte..byte + 1))
            },
        )))
    }
    fn up(
        &self,
        params: super::SelectionModeParams,
    ) -> Result<std::option::Option<Selection>, anyhow::Error> {
        self.move_vertically(true, params)
    }

    fn down(
        &self,
        params: super::SelectionModeParams,
    ) -> Result<std::option::Option<Selection>, anyhow::Error> {
        self.move_vertically(false, params)
    }

    fn first(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_char(params, SelectionPosition::First)
    }

    fn last(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        get_char(params, SelectionPosition::Last)
    }
}

fn line_len_without_new_line(current_line: &ropey::Rope) -> usize {
    let last_char_index = current_line.len_chars().saturating_sub(1);
    let last_char_is_newline = if let Some(chars) = current_line.get_chars_at(last_char_index) {
        chars.collect::<String>() == *"\n"
    } else {
        false
    };

    if last_char_is_newline {
        last_char_index
    } else {
        last_char_index.saturating_add(1)
    }
}

impl Character {
    fn move_vertically(
        &self,
        go_up: bool,
        super::SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
            ..
        }: super::SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        let current_char_index = current_selection.to_char_index(cursor_direction);
        let current_line = buffer.char_to_line(current_char_index)?;
        let line_index = if go_up {
            current_line.saturating_sub(1)
        } else {
            current_line.saturating_add(1)
        };
        let line_len = buffer
            .get_line_by_line_index(line_index)
            .map(|line| line_len_without_new_line(&Rope::from_str(&line.to_string())))
            .unwrap_or_default();
        let column = self.current_column.min(line_len.saturating_sub(1));
        let char_index =
            buffer.position_to_char(crate::position::Position::new(line_index, column))?;
        Ok(Some(Selection::new((char_index..char_index + 1).into())))
    }
}

fn get_char(
    params: super::SelectionModeParams,
    position: SelectionPosition,
) -> anyhow::Result<Option<crate::selection::Selection>> {
    if let Some(current_word) = Word::new(params.buffer, false)?.current(
        params.clone(),
        crate::components::editor::IfCurrentNotFound::LookForward,
    )? {
        let start = match position {
            SelectionPosition::First => current_word.range().start,
            SelectionPosition::Last => current_word.range().end - 1,
        };
        return Ok(Some(
            params
                .current_selection
                .clone()
                .set_range((start..start + 1).into()),
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod test_character {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() -> anyhow::Result<()> {
        let buffer = Buffer::new(None, "foo\nspam");

        // First line
        let selection = Selection::default();
        Character::new(0).assert_all_selections(
            &buffer,
            selection,
            &[(0..1, "f"), (1..2, "o"), (2..3, "o")],
        );

        // Second line
        let char_index = buffer.line_to_char(1)?;
        let selection = Selection::default().set_range((char_index..char_index).into());
        Character::new(0).assert_all_selections(
            &buffer,
            selection,
            &[(4..5, "s"), (5..6, "p"), (6..7, "a"), (7..8, "m")],
        );
        Ok(())
    }

    #[test]
    fn move_vertically() {
        let buffer = Buffer::new(
            None,
            "
alphz
  bete
   iodin
gam
  dlu  
"
            .trim(),
        );

        let test = |selected_line: usize, move_up: bool, expected: &str| {
            let start = buffer.line_to_char(selected_line).unwrap();
            let selection_mode = Character::new(4);
            let method = if move_up {
                Character::up
            } else {
                Character::down
            };
            let result = method(
                &selection_mode,
                crate::selection_mode::SelectionModeParams {
                    buffer: &buffer,
                    current_selection: &Selection::new((start..start + 1).into()),
                    cursor_direction: &crate::components::editor::Direction::Start,
                },
            )
            .unwrap()
            .unwrap();
            let actual = buffer.slice(&result.extended_range()).unwrap();
            assert_eq!(actual, expected);
        };

        let test_move_up =
            |selected_line: usize, expected: &str| test(selected_line, true, expected);

        test_move_up(1, "z");
        test_move_up(2, "t");
        test_move_up(3, "o");
        test_move_up(4, "m");

        let test_move_down =
            |selected_line: usize, expected: &str| test(selected_line, false, expected);
        test_move_down(0, "t");
        test_move_down(1, "o");
        test_move_down(2, "m");
        test_move_down(3, "u");
    }
}
