use regex::Regex;

use crate::position::Position;

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct WrappedLines {
    lines: Vec<WrappedLine>,
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

        let baseline = self
            .lines
            .get(position.line)
            .ok_or(CalibrationError::LineOutOfRange)?;

        let new_position = baseline
            .get_position(position.column)
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

    fn get_position(&self, column: usize) -> Option<Position> {
        // If this line is only made up of a newline character
        if self.primary.is_empty() && column == 0 {
            Some(Position {
                line: self.line_number,
                column: 0,
            })
        } else if column < self.primary.len() {
            Some(Position {
                line: self.line_number,
                column,
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
}

pub fn soft_wrap(text: &str, line_length: usize) -> WrappedLines {
    let re = Regex::new(r"\b").unwrap();
    let lines = text
        .lines()
        .enumerate()
        .filter_map(|(line_number, line)| {
            let wrapped_lines: Vec<String> = re.split(line).fold(vec![], |mut lines, word| {
                match lines.last_mut() {
                    Some(last_line) if last_line.len() + word.len() <= line_length => {
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
    WrappedLines { lines }
}

#[cfg(test)]
mod test_soft_wrap {

    #[cfg(test)]
    mod calibrate {
        use crate::position::Position;
        use crate::soft_wrap::soft_wrap;

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
    }
}
