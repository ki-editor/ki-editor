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
impl WrappedLines {
    /// The returned value is not one position but potentially multiple positions
    /// because some characters take multiple cells in terminal
    pub(crate) fn calibrate(&self, position: Position) -> Result<Vec<Position>, CalibrationError> {
        if self.lines.is_empty() && position.line == 0 && position.column == 0 {
            return Ok(vec![Position::new(0, 0)]);
        }

        if position.line == self.lines.len()
            && position.column == 0
            && self.ending_with_newline_character
        {
            return Ok(vec![Position::new(position.line, 0)]);
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

        Ok(new_positions
            .into_iter()
            .map(|new_position| {
                debug_assert!(new_position.column <= self.width);
                Position {
                    line: vertical_offset + new_position.line,
                    column: new_position.column,
                }
            })
            .collect_vec())
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

    fn get_positions(&self, column: usize, width: usize) -> Option<Vec<Position>> {
        let chars_with_line_index = &self.chars_with_line_index;
        if chars_with_line_index.is_empty() && column == 0 {
            return Some([Position::default()].to_vec());
        }
        if column > chars_with_line_index.len() {
            return None;
        }
        let (left, right) = chars_with_line_index.split_at(column);
        let line = right
            .split_first()
            .map(|((line, _), _)| line)
            .or_else(|| Some(&chars_with_line_index.last()?.0))?;
        let previous_columns_chars = left.iter().filter(|(line_, _)| line == line_).collect_vec();

        let char_width = right
            .first()
            .map(|(_, char)| get_char_width(*char))
            .unwrap_or(1);
        let previous_columns_chars_total_width: usize = get_string_width(
            &previous_columns_chars
                .into_iter()
                .map(|(_, char)| char)
                .join(""),
        );
        Some(
            (0..char_width)
                .map(|column| {
                    let calibrated_column = column + previous_columns_chars_total_width;
                    debug_assert!(calibrated_column <= width);
                    Position {
                        line: *line,
                        column: calibrated_column,
                    }
                })
                .collect_vec(),
        )
    }

    fn count(&self) -> usize {
        1 + self.wrapped.len()
    }
}

pub(crate) fn soft_wrap(text: &str, width: usize) -> WrappedLines {
    let re = Regex::new(r"\b").unwrap();

    // Need to reduce the width by 1 for wrapping,
    // that one space is reserved for rendering cursor at the last column
    let wrap_width = width.saturating_sub(1);
    let lines = text
        .lines()
        .enumerate()
        .filter_map(|(line_number, line)| {
            let wrapped_lines: Vec<String> = re
                .split(line)
                .flat_map(|line| chop_str(line, wrap_width))
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
                            _ => lines.push((chunk_width, chunk)),
                        }
                        lines
                    },
                )
                .into_iter()
                .map(|(_, line)| line)
                .collect_vec();
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
        result.to_string().replace("\n", ""),
        text.to_string().replace("\n", "")
    );
    result
}

fn chop_str(s: &str, max_width: usize) -> Vec<(usize, String)> {
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
    debug_assert_eq!(result.iter().map(|(_, s)| s).join(""), s);
    debug_assert!(result.iter().all(|(_, s)| get_string_width(s) <= max_width));
    result
}

#[cfg(test)]
mod test_soft_wrap {
    use crate::position::Position;

    use super::soft_wrap;
    use unicode_width::UnicodeWidthStr;

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
            wrapped_lines.calibrate(Position::new(0, 2)),
            Ok(vec![Position::new(1, 0)])
        );

        // The space character between the 👩 and 'abc'should be placed at first line, 3rd column
        assert_eq!(
            wrapped_lines.calibrate(Position::new(0, 1)),
            Ok(vec![Position::new(0, 2)])
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
                wrapped_lines.calibrate(Position::new(0, 0)),
                Ok([Position::new(0, 0), Position::new(0, 1)].to_vec()),
            );
        }

        #[test]
        fn ending_with_newline_char() {
            let content = "hello\n";
            let wrapped_lines = soft_wrap(content, 10);
            assert_eq!(
                wrapped_lines.calibrate(Position::new(1, 0)),
                Ok(vec![Position::new(1, 0)])
            );
        }

        #[test]
        fn normal() {
            fn assert(input: (usize, usize), expected: (usize, usize)) {
                let content = "hello world\nhey";
                let wrapped_lines = soft_wrap(content, 6);
                assert_eq!(
                    wrapped_lines.calibrate(Position::new(input.0, input.1)),
                    Ok(vec![Position::new(expected.0, expected.1),])
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
                wrapped_lines.calibrate(Position::new(1, 0)),
                Ok(vec![Position::new(1, 0)])
            );
        }

        #[test]
        fn no_wrap() {
            let content = "hello world\nhey";
            let wrapped_lines = soft_wrap(content, 100);

            assert_eq!(
                wrapped_lines.calibrate(Position::new(0, 0)),
                Ok(vec![Position::new(0, 0)])
            );

            assert_eq!(
                wrapped_lines.calibrate(Position::new(1, 0)),
                Ok(vec![Position::new(1, 0)])
            );
        }

        #[test]
        fn empty_content() {
            let content = "";
            let wrapped_lines = soft_wrap(content, 100);

            assert_eq!(
                wrapped_lines.calibrate(Position::new(0, 0)),
                Ok(vec![Position::new(0, 0)])
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
                wrapped_lines.calibrate(Position::new(0, 3)),
                Ok(vec![Position::new(0, 3)])
            );
        }

        #[test]
        fn column_longer_than_line_but_within_width_with_wrap() {
            let content = "hey jude";
            let wrapped_lines = soft_wrap(content, 5);

            assert_eq!(
                // Position one column before "jude"
                wrapped_lines.calibrate(Position::new(0, 4)),
                Ok(vec![Position::new(1, 0)])
            );

            assert_eq!(
                // Position one column after "jude"
                wrapped_lines.calibrate(Position::new(0, 8)),
                Ok(vec![Position::new(1, 4)])
            );
        }
    }
}
