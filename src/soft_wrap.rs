use itertools::Itertools;
use regex::Regex;

use crate::position::Position;

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

    /// Remove lines from the top until at least `wrapped_lines_count` is skipped.
    /// Returns the clamped `WrappedLines` and the number of lines skipped.
    pub(crate) fn skip_top(self, wrapped_lines_count: usize) -> (WrappedLines, usize) {
        let old_lines_len = self.lines.len();
        let new_lines = self
            .lines
            .into_iter()
            .scan(0, |cumulative_line_count, line| {
                let line_count = line.count();
                let result = (*cumulative_line_count, line);
                *cumulative_line_count += line_count;
                Some(result)
            })
            .skip_while(|(cumulative_line_count, _)| *cumulative_line_count < wrapped_lines_count)
            .map(|(_, line)| line)
            .collect_vec();
        let no_of_skipped_lines = old_lines_len.saturating_sub(new_lines.len());
        (
            WrappedLines {
                lines: new_lines
                    .into_iter()
                    .map(|line| WrappedLine {
                        // Re-adjust the line number of each WrappedLine
                        // So that calibration works as expected after `skip_top`
                        line_number: line.line_number.saturating_sub(no_of_skipped_lines),
                        ..line
                    })
                    .collect_vec(),
                ..self
            },
            no_of_skipped_lines,
        )
    }
}

#[derive(Debug, Clone)]
pub struct WrappedLine {
    /// 0-based
    line_number: usize,
    primary: String,
    wrapped: Vec<String>,
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
                    Some(last_line) if last_line.len() + word.len() <= width => {
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
    use crate::{position::Position, soft_wrap::soft_wrap};
    #[test]
    fn skip_top_1() {
        let content = "a ba\nc\nd";
        let wrapped_lines = soft_wrap(content, 3);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 4);
        let (clamped, no_of_skipped_lines) = wrapped_lines.skip_top(1);
        assert_eq!(clamped.wrapped_lines_count(), 2);
        assert_eq!(no_of_skipped_lines, 1);
        assert_eq!(
            clamped.calibrate(Position::new(0, 0)).unwrap(),
            Position::new(0, 0)
        );
    }

    #[test]
    fn skip_top_2() {
        let content = "a\nc de\nf";
        let wrapped_lines = soft_wrap(content, 3);
        assert_eq!(wrapped_lines.wrapped_lines_count(), 4);
        let (clamped, no_of_skipped_lines) = wrapped_lines.skip_top(1);
        assert_eq!(clamped.wrapped_lines_count(), 3);
        assert_eq!(no_of_skipped_lines, 1);
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
