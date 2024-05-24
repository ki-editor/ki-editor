use itertools::Itertools;

use crate::selection::CharIndex;

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub(crate) enum EnclosureKind {
    Parentheses,
    CurlyBraces,
    AngularBrackets,
    SquareBrackets,
    DoubleQuotes,
    SingleQuotes,
    Backticks,
}

/// Return the open index and close index of the given `kind`.
pub(crate) fn get_surrounding_indices(
    content: &str,
    kind: EnclosureKind,
    cursor_char_index: CharIndex,
) -> Option<(CharIndex, CharIndex)> {
    debug_assert!((0..content.chars().count()).contains(&cursor_char_index.0));
    if !(0..content.chars().count()).contains(&cursor_char_index.0) {
        return None;
    }
    let chars = content.chars().collect_vec();
    let (left, right) = {
        let (left, right) = chars.split_at(cursor_char_index.0);
        (left.to_vec(), right.to_vec())
    };
    let (open, close) = kind.open_close_symbols();
    fn get_index<I>(iter: I, encounter: Option<char>, target: char) -> Option<usize>
    where
        I: std::iter::Iterator<Item = (usize, char)>,
    {
        let mut count = 0;
        for (index, c) in iter {
            if Some(c) == encounter {
                count += 1;
            } else if c == target {
                if count > 0 {
                    count -= 1;
                } else {
                    return Some(index);
                }
            }
        }
        None
    }

    let open_index = if content.chars().nth(cursor_char_index.0) == Some(open) {
        cursor_char_index
    } else {
        let encounter = if open == close { None } else { Some(close) };
        cursor_char_index - (get_index(left.into_iter().rev().enumerate(), encounter, open)? + 1)
    };

    let close_index = if content.chars().nth(cursor_char_index.0) == Some(close) {
        cursor_char_index
    } else {
        let encounter = if open == close { None } else { Some(open) };
        cursor_char_index + (get_index(right.into_iter().enumerate().skip(1), encounter, close)?)
    };

    debug_assert_eq!(content.chars().nth(open_index.0), Some(open));

    debug_assert_eq!(content.chars().nth(close_index.0), Some(close));

    Some((open_index, close_index))
}

impl EnclosureKind {
    pub(crate) const fn open_close_symbols(&self) -> (char, char) {
        match self {
            EnclosureKind::Parentheses => ('(', ')'),
            EnclosureKind::CurlyBraces => ('{', '}'),
            EnclosureKind::AngularBrackets => ('<', '>'),
            EnclosureKind::SquareBrackets => ('[', ']'),
            EnclosureKind::DoubleQuotes => ('"', '"'),
            EnclosureKind::SingleQuotes => ('\'', '\''),
            EnclosureKind::Backticks => ('`', '`'),
        }
    }

    pub(crate) const fn open_close_symbols_str(&self) -> (&'static str, &'static str) {
        match self {
            EnclosureKind::Parentheses => ("(", ")"),
            EnclosureKind::CurlyBraces => ("{", "}"),
            EnclosureKind::AngularBrackets => ("<", ">"),
            EnclosureKind::SquareBrackets => ("[", "]"),
            EnclosureKind::DoubleQuotes => ("\"", "\""),
            EnclosureKind::SingleQuotes => ("'", "'"),
            EnclosureKind::Backticks => ("`", "`"),
        }
    }

    pub(crate) fn to_str(self) -> &'static str {
        match self {
            EnclosureKind::Parentheses => "Parentheses",
            EnclosureKind::CurlyBraces => "Curly Braces",
            EnclosureKind::AngularBrackets => "Angular Brackets",
            EnclosureKind::SquareBrackets => "Square Brackets",
            EnclosureKind::DoubleQuotes => "Double Quotes",
            EnclosureKind::SingleQuotes => "Single Quotes",
            EnclosureKind::Backticks => "Backticks",
        }
    }
}

#[cfg(test)]
mod test_surround {
    use super::*;
    fn run_test(
        content: &str,
        enclosure: EnclosureKind,
        cursor_char_index: usize,
        expected: Option<(usize, usize)>,
    ) {
        let actual = get_surrounding_indices(content, enclosure, CharIndex(cursor_char_index));
        assert_eq!(
            actual,
            expected.map(|(open, close)| (CharIndex(open), CharIndex(close)))
        )
    }

    use EnclosureKind::*;
    #[test]
    /// Cursor is within the open and close symbols, not on the open or close symbols
    fn test_get_surrounding_indices_1() {
        run_test("(hello)", Parentheses, 2, Some((0, 6)));
        run_test("(hello (world))", Parentheses, 2, Some((0, 14)));
        run_test("(hello (world))", Parentheses, 8, Some((7, 13)));
        run_test("(a (b) c)", Parentheses, 7, Some((0, 8)));
    }

    #[test]
    /// Cursor is on the open symbol
    fn test_get_surrounding_indices_2() {
        run_test("(hello)", Parentheses, 0, Some((0, 6)));
        run_test("(a (b))", Parentheses, 0, Some((0, 6)));
        run_test("(a (b))", Parentheses, 3, Some((3, 5)));
        run_test("(a (b (c)))", Parentheses, 3, Some((3, 9)));
    }

    #[test]
    /// Cursor is on the close symbol
    fn test_get_surrounding_indices_3() {
        run_test("(hello)", Parentheses, 6, Some((0, 6)));
        run_test("(a (b))", Parentheses, 6, Some((0, 6)));
        run_test("(a (b))", Parentheses, 5, Some((3, 5)));
        run_test("(a (b (c)))", Parentheses, 9, Some((3, 9)));
    }

    #[test]
    /// Open and close symbol are the same
    fn test_get_surrounding_indices_4() {
        run_test("'hello'", SingleQuotes, 2, Some((0, 6)));
    }
}
