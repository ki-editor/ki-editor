use crate::{
    char_index_range::{ToByteRange, ToCharIndexRange},
    selection::Selection,
};

use super::{SelectionMode, SelectionModeParams};

pub struct Inside(InsideKind);

impl SelectionMode for Inside {
    fn name(&self) -> &'static str {
        "INSIDE"
    }
    fn iter<'a>(
        &'a self,
        params: SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(std::iter::empty()))
    }

    fn current(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            ..
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        let (open, close) = self.0.open_close_symbols();
        let range = current_selection.extended_range();
        let start = buffer.find_nearest_string_before(range.start, &open);
        let end = buffer.find_nearest_string_after(range.end, &close);

        Ok(Some(match (start, end) {
            (Some(start), Some(end)) => current_selection.clone().set_range((start..end).into()),
            _ => current_selection.clone(),
        }))
    }

    fn up(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        let SelectionModeParams {
            buffer,
            current_selection,
            ..
        } = params;
        let text = params.selected_text()?;
        let (open, close) = self.0.open_close_symbols();
        if text.starts_with(&open) && text.ends_with(&close) {
            return self.current(params);
        }
        let range = current_selection.range().to_byte_range(buffer)?;
        let start = range.start.saturating_sub(open.len());
        let end = range.end.saturating_add(close.len());
        Ok(Some(
            current_selection
                .clone()
                .set_range((start..end).to_char_index_range(buffer)?),
        ))
    }
    fn down(&self, params: SelectionModeParams) -> anyhow::Result<Option<Selection>> {
        let SelectionModeParams {
            buffer,
            current_selection,
            ..
        } = params;
        let (open, close) = self.0.open_close_symbols();
        let text = params.selected_text()?;
        let range = current_selection.extended_range();

        if text.starts_with(&open) && text.ends_with(&close) {
            let range = range.to_byte_range(buffer)?;
            let start = range.start.saturating_add(open.len());
            let end = range.end.saturating_sub(close.len());

            Ok(Some(
                current_selection
                    .clone()
                    .set_range((start..end).to_char_index_range(buffer)?),
            ))
        } else {
            let start = buffer.find_nearest_string_after(range.start, &open);
            let end = buffer.find_nearest_string_before(range.end, &close);
            Ok(Some(match (start, end) {
                (Some(start), Some(end)) => {
                    let end = buffer
                        .byte_to_char(buffer.char_to_byte(end)?.saturating_add(close.len()))?
                        - 1;
                    current_selection.clone().set_range((start..end).into())
                }
                _ => current_selection.clone(),
            }))
        }
    }
}

#[derive(PartialEq, Clone, Debug, Eq)]
pub enum InsideKind {
    Parentheses,
    CurlyBraces,
    AngularBrackets,
    SquareBrackets,
    DoubleQuotes,
    SingleQuotes,
    BackQuotes,
    Custom(char),
}

impl InsideKind {
    fn to_string(&self) -> String {
        match self {
            InsideKind::Parentheses => "Parentheses".to_string(),
            InsideKind::CurlyBraces => "Curly Braces".to_string(),
            InsideKind::AngularBrackets => "Angular Brackets".to_string(),
            InsideKind::SquareBrackets => "Square Brackets".to_string(),
            InsideKind::DoubleQuotes => "Double Quotes".to_string(),
            InsideKind::SingleQuotes => "Single Quotes".to_string(),
            InsideKind::BackQuotes => "Back Quotes".to_string(),
            InsideKind::Custom(char) => format!("'{}'", char),
        }
    }

    fn open_close_symbols(&self) -> (String, String) {
        match self {
            InsideKind::Parentheses => ("(".to_string(), ")".to_string()),
            InsideKind::CurlyBraces => ("{".to_string(), "}".to_string()),
            InsideKind::AngularBrackets => ("<".to_string(), ">".to_string()),
            InsideKind::SquareBrackets => ("[".to_string(), "]".to_string()),
            InsideKind::DoubleQuotes => ("\"".to_string(), "\"".to_string()),
            InsideKind::SingleQuotes => ("'".to_string(), "'".to_string()),
            InsideKind::BackQuotes => ("`".to_string(), "`".to_string()),
            InsideKind::Custom(char) => (char.to_string(), char.to_string()),
        }
    }
}

#[cfg(test)]
mod test_inside {

    use crate::{buffer::Buffer, components::editor::Movement, selection::CharIndex};

    use super::*;

    #[test]
    fn up() -> anyhow::Result<()> {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a b (c (d e))");
        let inside = Inside(InsideKind::Parentheses);

        let ups = inside.generate_selections(
            &buffer,
            Movement::Up,
            4,
            (CharIndex(8)..CharIndex(11)).into(),
        )?;
        assert_eq!(ups, &["(d e)", "c (d e)", "(c (d e))", "(c (d e))"]);

        let downs = inside.generate_selections(
            &buffer,
            Movement::Down,
            4,
            (CharIndex(4)..CharIndex(13)).into(),
        )?;
        assert_eq!(downs, &["c (d e)", "(d e)", "d e", "d e"]);

        Ok(())
    }
}
