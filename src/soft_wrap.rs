use std::fmt::Display;

use itertools::Itertools;
use regex::Regex;

use crate::{
    grid::{get_char_width, get_string_width},
    position::Position,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct WrappedLines {
    width: usize,
    lines: Vec<WrappedLine>,
    ending_with_newline_character: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) enum CalibrationError {
    LineOutOfRange,
    ColumnOutOfRange,
}
impl Display for WrappedLines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.lines
                .iter()
                .map(|line| line.to_string())
                .collect_vec()
                .join("\n")
        )
    }
}

pub(crate) struct Positions(Box<dyn Iterator<Item = Position>>);

impl Positions {
    pub(crate) fn into_iter(self) -> Box<dyn Iterator<Item = Position>> {
        self.0
    }

    pub(crate) fn first(&mut self) -> Option<Position> {
        self.0.next()
    }

    fn single(position: Position) -> Positions {
        Positions(Box::new(std::iter::once(position)))
    }

    #[cfg(test)]
    fn into_vec(self) -> Vec<Position> {
        self.into_iter().collect_vec()
    }
}
impl WrappedLines {
    /// The returned value is not one position but potentially multiple positions
    /// because some characters take multiple cells in terminal
    pub(crate) fn calibrate(&self, position: Position) -> Result<Positions, CalibrationError> {
        if self.lines.is_empty() && position.line == 0 && position.column == 0 {
            return Ok(Positions::single(Position::new(0, 0)));
        }

        if position.line == self.lines.len()
            && position.column == 0
            && self.ending_with_newline_character
        {
            return Ok(Positions::single(Position::new(position.line, 0)));
        }

        let baseline = self
            .lines
            .get(position.line)
            .ok_or(CalibrationError::LineOutOfRange)?;

        let new_positions = baseline
            .get_positions(position.column, self.width)
            .ok_or(CalibrationError::ColumnOutOfRange)?;

        let vertical_offset = {
            let previous_lines = self.lines.iter().take(position.line);
            previous_lines.map(|line| line.count()).sum::<usize>()
        };

        let width = self.width;

        Ok(Positions(Box::new(new_positions.into_iter().map(
            move |new_position| {
                debug_assert!(new_position.column <= width);
                Position {
                    line: vertical_offset + new_position.line,
                    column: new_position.column,
                }
            },
        ))))
    }

    pub(crate) fn lines(&self) -> &Vec<WrappedLine> {
        &self.lines
    }

    pub(crate) fn wrapped_lines_count(&self) -> usize {
        self.lines.iter().map(|line| line.count()).sum()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WrappedLine {
    /// 0-based
    line_number: usize,
    primary: String,
    wrapped: Vec<String>,
    /// This can be computed on demand, but it is stored as cache to
    /// greatly improve the performace of `WrappedLines::calibrate`
    chars_with_line_index: Vec<(usize /* line index (0-based) */, char)>,
}
impl Display for WrappedLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lines().join("\n"))
    }
}
impl WrappedLine {
    pub(crate) fn lines(&self) -> Vec<String> {
        [self.primary.clone()]
            .into_iter()
            .chain(self.wrapped.iter().cloned())
            .collect()
    }

    pub(crate) fn line_number(&self) -> usize {
        self.line_number
    }

    fn get_positions(&self, column: usize, width: usize) -> Option<Positions> {
        let chars_with_line_index = &self.chars_with_line_index;
        if chars_with_line_index.is_empty() && column == 0 {
            return Some(Positions::single(Position::default()));
        }
        if column > chars_with_line_index.len() {
            return None;
        }
        let (left, right) = chars_with_line_index.split_at(column);
        let line = right
            .split_first()
            .map(|((line, _), _)| *line)
            .or_else(|| Some(chars_with_line_index.last()?.0))?;
        let previous_columns_chars = left.iter().filter(|(line_, _)| &line == line_);

        let char_width = right
            .first()
            .map(|(_, char)| get_char_width(*char))
            .unwrap_or(1);
        let previous_columns_chars_total_width: usize = previous_columns_chars
            .map(move |(_, char)| get_char_width(*char))
            .sum();
        Some(Positions(Box::new((0..char_width).map(move |column| {
            let calibrated_column = column + previous_columns_chars_total_width;
            debug_assert!(calibrated_column <= width);
            Position {
                line,
                column: calibrated_column,
            }
        }))))
    }

    fn count(&self) -> usize {
        1 + self.wrapped.len()
    }
}

pub(crate) fn soft_wrap(text: &str, width: usize) -> WrappedLines {
    let re = Regex::new(r"\b").unwrap();

    // LABEL: NEED_TO_REDUCE_WIDTH_BY_1
    // Need to reduce the width by 1 for wrapping,
    // that one space is reserved for rendering cursor at the last column
    let wrap_width = width.saturating_sub(1);
    let lines = text
        .lines()
        .enumerate()
        .filter_map(|(line_number, line)| {
            let items = re.split(line).collect_vec();
            let wrapped_lines: Vec<String> = wrap_items(&items, wrap_width);
            let (primary, wrapped) = wrapped_lines.split_first()?;
            Some(WrappedLine {
                primary: primary.to_string(),
                line_number,
                wrapped: wrapped.to_vec(),
                chars_with_line_index: wrapped_lines
                    .into_iter()
                    .enumerate()
                    .flat_map(|(line_index, line)| {
                        line.chars().map(|char| (line_index, char)).collect_vec()
                    })
                    .collect_vec(),
            })
        })
        .collect();
    let result = WrappedLines {
        lines,
        width,
        ending_with_newline_character: text.ends_with('\n'),
    };
    debug_assert_eq!(
        result.to_string().replace('\n', ""),
        text.to_string().replace('\n', "")
    );
    result
}

pub(crate) fn wrap_items(items: &[&str], wrap_width: usize) -> Vec<String> {
    debug_assert!(wrap_width > 0);
    items
        .into_iter()
        .flat_map(|chunk| chop_str(chunk, wrap_width))
        .fold(
            vec![],
            |mut lines: Vec<(usize, String)>, (chunk_width, chunk)| {
                match lines.last_mut() {
                    Some((last_line_width, last_line))
                        if *last_line_width + chunk_width <= wrap_width =>
                    {
                        last_line.push_str(&chunk);
                        *last_line_width += chunk_width;
                    }
                    _ => lines.push((chunk_width, chunk.to_string())),
                }
                lines
            },
        )
        .into_iter()
        .map(|(_, line)| line)
        .collect_vec()
}

/// Chop the given string into chunks by the given `max_width`
/// The width of each chunk is paired with each chunk in the result vector.
fn chop_str(s: &str, max_width: usize) -> Vec<(usize, String)> {
    debug_assert!(max_width > 0);
    fn chop_str_(s: &str, max_width: usize) -> Vec<(usize, String)> {
        let width = get_string_width(s);
        if width <= max_width {
            return vec![(width, s.to_string())];
        }
        let mut result = vec![];
        let mut current = vec![];
        let mut current_width = 0;
        for c in s.chars() {
            let char_width = get_char_width(c);
            if char_width + current_width <= max_width {
                current.push(c);
                current_width += char_width;
            } else {
                result.push((current_width, current.drain(..).join("")));
                current_width = char_width;
                current = vec![c];
            }
        }
        if !current.is_empty() {
            result.push((current_width, current.drain(..).join("")));
        }

        result
    }
    let result = chop_str_(s, max_width);
    debug_assert!(if get_string_width(s) <= max_width {
        result.len() == 1
    } else {
        result.len() > 1
    });
    debug_assert_eq!(result.iter().map(|(_, s)| s).join(""), s);
    debug_assert!(result.iter().all(|(_, s)| get_string_width(s) <= max_width));
    debug_assert_eq!(
        result
            .iter()
            .map(|(_, s)| get_string_width(s))
            .sum::<usize>(),
        get_string_width(s)
    );
    debug_assert_eq!(
        result.iter().map(|(width, _)| width).sum::<usize>(),
        get_string_width(s)
    );
    result
}

#[cfg(test)]
mod test_soft_wrap {
    use crate::position::Position;

    use super::{chop_str, soft_wrap};
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn test_chop_str() {
        assert_eq!(chop_str("hello", 6), vec![(5, "hello".to_string())]);
        assert_eq!(chop_str("", 6), vec![(0, "".to_string())]);
        assert_eq!(
            chop_str("spongebob", 6),
            vec![(6, "sponge".to_string()), (3, "bob".to_string())]
        );
        assert_eq!(
            chop_str("\t\t", 6),
            vec![(4, "\t".to_string()), (4, "\t".to_string())]
        )
    }

    #[test]
    fn consider_unicode_width_1() {
        let content = "→ abc";
        let wrapped_lines = soft_wrap(content, content.chars().count() + 1);
        assert_eq!(UnicodeWidthStr::width("→"), 1);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 1)
    }

    #[test]
    /// Line with emoji: wrapped
    fn consider_unicode_width_2() {
        let content = "👩 abc";
        let wrapped_lines = soft_wrap(content, content.chars().count() + 1);
        assert_eq!(UnicodeWidthStr::width("👩"), 2);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 2);

        // The character 'a' should be placed at the next line, first column
        assert_eq!(
            wrapped_lines
                .calibrate(Position::new(0, 2))
                .unwrap()
                .into_vec(),
            vec![Position::new(1, 0)]
        );

        // The space character between the 👩 and 'abc'should be placed at first line, 3rd column
        assert_eq!(
            wrapped_lines
                .calibrate(Position::new(0, 1))
                .unwrap()
                .into_vec(),
            vec![Position::new(0, 2)]
        );
    }

    #[test]
    fn hard_wrap_word_longer_than_container_width() {
        let content = "spongebob";
        let wrapped_lines = soft_wrap(content, 6);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 2);
        assert_eq!(wrapped_lines.to_string(), "spong\nebob")
    }

    #[test]
    fn consider_tab_width_1() {
        let content = "\tabc";
        let wrapped_lines = soft_wrap(content, 5);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 2)
    }

    #[test]
    fn wrap_width_should_be_one_less_than_container_width() {
        let content = "a ba";
        let wrapped_lines = soft_wrap(content, content.len());

        // Although the container width is same as the content length,
        // the content is still wrapped, because `wrap_width = container_width - 1`.
        assert_eq!(wrapped_lines.wrapped_lines_count(), 2);
    }

    #[cfg(test)]
    mod calibrate {

        use crate::position::Position;
        use crate::soft_wrap::soft_wrap;

        #[test]
        fn multi_width_unicode_should_be_padded() {
            let content = "🦀";
            let wrapped_lines = soft_wrap(content, 10);
            assert_eq!(
                wrapped_lines
                    .calibrate(Position::new(0, 0))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(0, 0), Position::new(0, 1)]
            );
        }

        #[test]
        fn ending_with_newline_char() {
            let content = "hello\n";
            let wrapped_lines = soft_wrap(content, 10);
            assert_eq!(
                wrapped_lines
                    .calibrate(Position::new(1, 0))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(1, 0)]
            );
        }

        #[test]
        fn normal() {
            fn assert(input: (usize, usize), expected: (usize, usize)) {
                let content = "hello world\nhey";
                let wrapped_lines = soft_wrap(content, 6);
                assert_eq!(
                    wrapped_lines
                        .calibrate(Position::new(input.0, input.1))
                        .unwrap()
                        .into_vec(),
                    vec![Position::new(expected.0, expected.1),]
                );
            }

            assert((0, 0), (0, 0));
            assert((0, 1), (0, 1));

            assert((0, 5), (1, 0));
            assert((0, 6), (2, 0));

            assert((1, 0), (3, 0));
            assert((1, 1), (3, 1));
        }

        #[test]
        fn empty_line() {
            let content = "hello world\n\n\nhey\n\nlol";
            let wrapped_lines = soft_wrap(content, 100);

            assert_eq!(
                wrapped_lines
                    .calibrate(Position::new(1, 0))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(1, 0)]
            );
        }

        #[test]
        fn no_wrap() {
            let content = "hello world\nhey";
            let wrapped_lines = soft_wrap(content, 100);

            assert_eq!(
                wrapped_lines
                    .calibrate(Position::new(0, 0))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(0, 0)]
            );

            assert_eq!(
                wrapped_lines
                    .calibrate(Position::new(1, 0))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(1, 0)]
            );
        }

        #[test]
        fn empty_content() {
            let content = "";
            let wrapped_lines = soft_wrap(content, 100);

            assert_eq!(
                wrapped_lines
                    .calibrate(Position::new(0, 0))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(0, 0)]
            );
        }

        #[test]
        /// This case is necesarry for the cursor to be able to move to the end of the line in
        /// Insert mode
        fn column_longer_than_line_but_within_width_without_wrap() {
            let content = "hey";
            let wrapped_lines = soft_wrap(content, 5);

            assert_eq!(
                // Position one column after "hey"
                wrapped_lines
                    .calibrate(Position::new(0, 3))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(0, 3)]
            );
        }

        #[test]
        fn column_longer_than_line_but_within_width_with_wrap() {
            let content = "hey jude";
            let wrapped_lines = soft_wrap(content, 5);

            assert_eq!(
                // Position one column before "jude"
                wrapped_lines
                    .calibrate(Position::new(0, 4))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(1, 0)]
            );

            assert_eq!(
                // Position one column after "jude"
                wrapped_lines
                    .calibrate(Position::new(0, 8))
                    .unwrap()
                    .into_vec(),
                vec![Position::new(1, 4)]
            );
        }
    }
}
