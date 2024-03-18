use crate::selection_mode::ApplyMovementResult;
use itertools::Itertools;

use crate::{
    char_index_range::{CharIndexRange, ToByteRange, ToCharIndexRange},
    selection::Selection,
};

use super::{SelectionMode, SelectionModeParams};

pub struct Inside(pub InsideKind);

impl Inside {
    pub fn new(kind: InsideKind) -> Self {
        Self(kind)
    }
}

impl SelectionMode for Inside {
    fn name(&self) -> &'static str {
        "INSIDE"
    }
    fn iter<'a>(
        &'a self,
        _: SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(std::iter::empty()))
    }

    fn current(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
            ..
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<Selection>> {
        let (open, close) = self.0.open_close_symbols();
        let range = current_selection.get_anchor(cursor_direction);
        let pair = buffer.find_nearest_pair((range..range).into(), &open, &close);
        Ok(pair.map(|pair| current_selection.clone().set_range(pair.inner_range())))
    }

    fn parent(&self, params: SelectionModeParams) -> anyhow::Result<Option<ApplyMovementResult>> {
        let SelectionModeParams {
            current_selection, ..
        } = params;
        let text = params.selected_text()?;

        let (open, close) = self.0.open_close_symbols();
        let range = current_selection.extended_range();
        if text.starts_with(&open) && text.ends_with(&close) {
            return Ok(self
                .current(params)?
                .map(ApplyMovementResult::from_selection));
        }

        let start = range.start - open.chars().count();
        let end = range.end + close.chars().count();

        let range: CharIndexRange = (start..end).into();

        Ok(Some(ApplyMovementResult::from_selection(
            current_selection.clone().set_range(range),
        )))
    }

    fn first_child(
        &self,
        params: SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        let SelectionModeParams {
            buffer,
            current_selection,
            ..
        } = params;
        let (open, close) = self.0.open_close_symbols();
        let text = params.selected_text()?;

        let range = current_selection.extended_range();

        let range = if text.starts_with(&open) && text.ends_with(&close) {
            let range = range.to_byte_range(buffer)?;
            let start = range.start.saturating_add(open.len());
            let end = range.end.saturating_sub(close.len());

            Some((start..end).to_char_index_range(buffer)?)
        } else {
            buffer
                .find_pairs(&open, &close)
                .into_iter()
                .sorted_by_key(|pair| pair.open.char_index_range.start)
                .find(|pair| {
                    let outer_range = pair.outer_range();
                    range.start <= outer_range.start && outer_range.end <= range.end
                })
                .map(|pair| pair.outer_range())
        };
        Ok(range.map(|range| {
            ApplyMovementResult::from_selection(current_selection.clone().set_range(range))
        }))
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
    Backtick,
    Other { open: String, close: String },
}

impl std::fmt::Display for InsideKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InsideKind::Parentheses => "Parentheses".to_string(),
                InsideKind::CurlyBraces => "Curly Braces".to_string(),
                InsideKind::AngularBrackets => "Angular Brackets".to_string(),
                InsideKind::SquareBrackets => "Square Brackets".to_string(),
                InsideKind::DoubleQuotes => "Double Quotes".to_string(),
                InsideKind::SingleQuotes => "Single Quotes".to_string(),
                InsideKind::Backtick => "Backticks".to_string(),
                InsideKind::Other { open, close } => format!("'{} {}'", open, close),
            }
        )
    }
}

impl InsideKind {
    fn open_close_symbols(&self) -> (String, String) {
        match self {
            InsideKind::Parentheses => ("(".to_string(), ")".to_string()),
            InsideKind::CurlyBraces => ("{".to_string(), "}".to_string()),
            InsideKind::AngularBrackets => ("<".to_string(), ">".to_string()),
            InsideKind::SquareBrackets => ("[".to_string(), "]".to_string()),
            InsideKind::DoubleQuotes => ("\"".to_string(), "\"".to_string()),
            InsideKind::SingleQuotes => ("'".to_string(), "'".to_string()),
            InsideKind::Backtick => ("`".to_string(), "`".to_string()),
            InsideKind::Other { open, close } => (open.clone(), close.clone()),
        }
    }
}

#[cfg(test)]
mod test_inside {

    use crate::{
        buffer::Buffer,
        components::editor::{Direction, Movement},
        context::Context,
        selection::{CharIndex, Filters},
    };

    use super::*;

    #[test]
    fn current_open_close_same() -> anyhow::Result<()> {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a b 'c 'd e''");
        let inside = Inside(InsideKind::Other {
            open: "'".to_string(),
            close: "'".to_string(),
        });
        let params = SelectionModeParams {
            buffer: &buffer,
            current_selection: &Selection::default(),
            cursor_direction: &Direction::default(),
            context: &Context::default(),
            filters: &Filters::default(),
        };

        let current = inside.current(SelectionModeParams {
            current_selection: &Selection::default().set_range((CharIndex(5)..CharIndex(6)).into()),
            ..params
        })?;
        let current_text = buffer.slice(&current.unwrap().extended_range())?;
        assert_eq!(current_text, "c ");

        let current = inside.current(SelectionModeParams {
            current_selection: &Selection::default()
                .set_range((CharIndex(9)..CharIndex(10)).into()),
            ..params
        })?;
        assert!(current.is_none());
        Ok(())
    }

    #[test]
    fn current_open_close_different() -> anyhow::Result<()> {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a b {|c {|d e|}|}");
        let inside = Inside(InsideKind::Other {
            open: "{|".to_string(),
            close: "|}".to_string(),
        });

        let test_cases = &[(6..7, "c {|d e|}"), (8..10, "c {|d e|}")];
        for (range, expected) in test_cases {
            let current = inside.current(SelectionModeParams {
                buffer: &buffer,
                current_selection: &Selection::default()
                    .set_range((CharIndex(range.start)..CharIndex(range.end)).into()),
                cursor_direction: &Direction::default(),
                context: &Context::default(),
                filters: &Filters::default(),
            })?;
            let current_text = buffer.slice(&current.unwrap().extended_range())?;
            assert_eq!(current_text.to_string(), expected.to_string());
        }

        Ok(())
    }

    #[test]
    fn parent() -> anyhow::Result<()> {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a b {|c {|d e|}|}");
        let inside = Inside(InsideKind::Other {
            open: "{|".to_string(),
            close: "|}".to_string(),
        });

        let ups = inside.generate_selections(
            &buffer,
            Movement::Parent,
            3,
            (CharIndex(10)..CharIndex(13)).into(),
        )?;
        assert_eq!(ups, &["{|d e|}", "c {|d e|}", "{|c {|d e|}|}"]);

        Ok(())
    }

    #[test]
    fn first_child() -> anyhow::Result<()> {
        let buffer = Buffer::new(tree_sitter_rust::language(), "a b {|c {|d e|}|} {| x |}");
        let inside = Inside(InsideKind::Other {
            open: "{|".to_string(),
            close: "|}".to_string(),
        });

        let downs = inside.generate_selections(
            &buffer,
            Movement::FirstChild,
            3,
            (CharIndex(4)..CharIndex(17)).into(),
        )?;
        assert_eq!(downs, &["c {|d e|}", "{|d e|}", "d e"]);

        Ok(())
    }
}
