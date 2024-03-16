use std::fmt::Display;

use itertools::Itertools;
use regex::Regex;

use crate::{grid::get_string_width, position::Position};

#[derive(Debug, Clone, Default)]
pub struct WrappedLines {
    width: usize,
    lines: Vec<WrappedLine>,
    ending_with_newline_character: bool,
}

#[derive(Debug, PartialEq)]
pub enum CalibrationError {
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
    pub fn calibrate(&self, position: Position) -> Result<Position, CalibrationError> {
        if self.lines.is_empty() && position.line == 0 && position.column == 0 {
            return Ok(Position::new(0, 0));
        }

        if position.line == self.lines.len()
            && position.column == 0
            && self.ending_with_newline_character
        {
            return Ok(Position::new(position.line, 0));
        }

        let baseline = self
            .lines
            .get(position.line)
            .ok_or(CalibrationError::LineOutOfRange)?;

        let new_position = baseline
            .get_position(position.column, self.width)
            .ok_or(CalibrationError::ColumnOutOfRange)?;

        let vertical_offset = {
            let previous_lines = self.lines.iter().take(position.line);
            previous_lines.map(|line| line.wrapped.len()).sum::<usize>()
        };

        Ok(Position {
            line: vertical_offset + new_position.line,
            column: new_position.column,
        })
    }

    pub fn lines(&self) -> &Vec<WrappedLine> {
        &self.lines
    }

    pub(crate) fn wrapped_lines_count(&self) -> usize {
        self.lines.iter().map(|line| line.count()).sum()
    }
}

#[derive(Debug, Clone)]
pub struct WrappedLine {
    /// 0-based
    line_number: usize,
    primary: String,
    wrapped: Vec<String>,
}
impl Display for WrappedLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lines().join("\n"))
    }
}
impl WrappedLine {
    pub fn lines(&self) -> Vec<String> {
        [self.primary.clone()]
            .into_iter()
            .chain(self.wrapped.iter().cloned())
            .collect()
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }

    fn get_position(&self, column: usize, width: usize) -> Option<Position> {
        // If the column is within the primary line
        // or if the line is not wrapped and the column is within the width
        if column < self.primary.len() || self.wrapped.is_empty() && column < width {
            Some(Position {
                line: self.line_number,
                column,
            })
        }
        // If the column is longer than this line but it's wrapped column is within the width
        else if column >= self.len() && column - self.len() < width {
            Some(Position {
                line: self.line_number + self.wrapped.len(),
                column: column - self.len() + self.last_line().len(),
            })
        } else {
            let mut column = column - self.primary.len();
            for (line_number, line) in self.wrapped.iter().enumerate() {
                if column < line.len() {
                    return Some(Position {
                        line: self.line_number + line_number + 1,
                        column,
                    });
                }
                column -= line.len();
            }
            None
        }
    }

    fn len(&self) -> usize {
        self.primary.len() + self.wrapped.iter().map(|line| line.len()).sum::<usize>()
    }

    fn last_line(&self) -> String {
        self.wrapped.last().unwrap_or(&self.primary).to_string()
    }

    fn count(&self) -> usize {
        1 + self.wrapped.len()
    }
}

pub fn soft_wrap(text: &str, width: usize) -> WrappedLines {
    let re = Regex::new(r"\b").unwrap();
    let lines = text
        .lines()
        .enumerate()
        .filter_map(|(line_number, line)| {
            let wrapped_lines: Vec<String> = re.split(line).fold(vec![], |mut lines, word| {
                match lines.last_mut() {
                    Some(last_line)
                        if get_string_width(last_line.as_str()) + get_string_width(word)
                            <= width =>
                    {
                        last_line.push_str(word);
                    }
                    _ => lines.push(word.to_string()),
                }
                lines
            });
            let (primary, wrapped) = wrapped_lines.split_first()?;
            Some(WrappedLine {
                primary: primary.to_string(),
                line_number,
                wrapped: wrapped.to_vec(),
            })
        })
        .collect();
    WrappedLines {
        lines,
        width,
        ending_with_newline_character: text.ends_with('\n'),
    }
}

#[cfg(test)]
mod test_soft_wrap {
    use super::soft_wrap;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn consider_unicode_width_1() {
        let content = "→ abc";
        let wrapped_lines = soft_wrap(content, 5);
        assert_eq!(UnicodeWidthStr::width("→"), 1);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 1)
    }

    #[test]
    fn consider_unicode_width_2() {
        let content = "👩 abc";
        let wrapped_lines = soft_wrap(content, 5);
        assert_eq!(UnicodeWidthStr::width("👩"), 2);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 2)
    }

    #[test]
    fn consider_tab_width() {
        let content = "\tabc";
        let wrapped_lines = soft_wrap(content, 5);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 2)
    }

    #[cfg(test)]
    mod calibrate {
        use crate::position::Position;
        use crate::soft_wrap::soft_wrap;

        #[test]
        fn ending_with_newline_char() {
            let content = "hello\n";
            let wrapped_lines = soft_wrap(content, 10);
            assert_eq!(
                wrapped_lines.calibrate(Position::new(1, 0)),
                Ok(Position::new(1, 0))
            );
        }

        #[test]
        fn normal() {
            fn assert(input: (usize, usize), expected: (usize, usize)) {
                let content = "hello world\nhey";
                let wrapped_lines = soft_wrap(content, 5);
                assert_eq!(
                    wrapped_lines.calibrate(Position::new(input.0, input.1)),
                    Ok(Position::new(expected.0, expected.1),)
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
                Ok(Position::new(1, 0))
            );
        }

        #[test]
        fn no_wrap() {
            let content = "hello world\nhey";
            let wrapped_lines = soft_wrap(content, 100);

            assert_eq!(
                wrapped_lines.calibrate(Position::new(0, 0)),
                Ok(Position::new(0, 0))
            );

            assert_eq!(
                wrapped_lines.calibrate(Position::new(1, 0)),
                Ok(Position::new(1, 0))
            );
        }

        #[test]
        fn empty_content() {
            let content = "";
            let wrapped_lines = soft_wrap(content, 100);

            assert_eq!(
                wrapped_lines.calibrate(Position::new(0, 0)),
                Ok(Position::new(0, 0))
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
                Ok(Position::new(0, 3))
            );
        }

        #[test]
        fn column_longer_than_line_but_within_width_with_wrap() {
            let content = "hey jude";
            let wrapped_lines = soft_wrap(content, 5);

            assert_eq!(
                // Position one column before "jude"
                wrapped_lines.calibrate(Position::new(0, 4)),
                Ok(Position::new(1, 0))
            );

            assert_eq!(
                // Position one column after "jude"
                wrapped_lines.calibrate(Position::new(0, 8)),
                Ok(Position::new(1, 4))
            );
        }
    }
}
