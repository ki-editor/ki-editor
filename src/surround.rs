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

impl std::fmt::Display for EnclosureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnclosureKind::Parentheses => write!(f, "Parentheses"),
            EnclosureKind::CurlyBraces => write!(f, "CurlyBraces"),
            EnclosureKind::AngularBrackets => write!(f, "AngularBrackets"),
            EnclosureKind::SquareBrackets => write!(f, "SquareBrackets"),
            EnclosureKind::DoubleQuotes => write!(f, "DoubleQuotes"),
            EnclosureKind::SingleQuotes => write!(f, "SingleQuotes"),
            EnclosureKind::Backticks => write!(f, "Backticks"),
        }
    }
}

/// Return the open index and close index of the given `kind`.
pub(crate) fn get_surrounding_indices(
    content: &str,
    kind: EnclosureKind,
    cursor_char_index: CharIndex,
    include_cursor_position: bool,
) -> Option<(CharIndex, CharIndex)> {
    debug_assert!((0..content.chars().count()).contains(&cursor_char_index.0));
    if !(0..content.chars().count()).contains(&cursor_char_index.0) {
        return None;
    }
    let chars = content.chars().collect_vec();
    let (open, close) = kind.open_close_symbols();
    let open_index = {
        let index = if include_cursor_position {
            cursor_char_index + 1
        } else {
            cursor_char_index
        }
        .0;
        let mut count = 0;
        let Some(open_index) =
            chars[0..index]
                .into_iter()
                .enumerate()
                .rev()
                .find(|(index, char)| {
                    if *char == &close && open != close {
                        count += 1
                    } else if *char == &open {
                        if count > 0 {
                            count -= 1
                        } else {
                            return true;
                        }
                    }
                    return false;
                })
        else {
            return None;
        };
        CharIndex(open_index.0)
    };
    let close_index = {
        let start_index = open_index.0 + 1;
        let mut count = 0;
        let Some(close_index) = chars[start_index..]
            .into_iter()
            .enumerate()
            .find(|(_, char)| {
                if *char == &open && open != close {
                    count += 1
                } else if *char == &close {
                    if count > 0 {
                        count -= 1
                    } else {
                        return true;
                    }
                }
                return false;
            })
        else {
            return None;
        };
        let close_index = close_index.0 + start_index;
        CharIndex(close_index)
    };
    return Some((open_index, close_index));
    let (left, right) = {
        let (left, right) = chars.split_at(cursor_char_index.0);
        (left.to_vec(), right.to_vec())
    };
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

    let open_index = if include_cursor_position
        && content.chars().nth(cursor_char_index.0) == Some(open)
    {
        cursor_char_index
    } else {
        let encounter = if open == close { None } else { Some(close) };
        cursor_char_index - (get_index(left.into_iter().rev().enumerate(), encounter, open)? + 1)
    };
    let close_index = if include_cursor_position
        && content.chars().nth(cursor_char_index.0) == Some(close)
    {
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
    pub(crate) const fn open_symbol(&self) -> char {
        self.open_close_symbols().0
    }

    pub(crate) const fn close_symbol(&self) -> char {
        self.open_close_symbols().1
    }

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

    pub(crate) const fn is_both_end_same(&self) -> bool {
        self.open_symbol() == self.close_symbol()
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
        let actual =
            get_surrounding_indices(content, enclosure, CharIndex(cursor_char_index), true);
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
