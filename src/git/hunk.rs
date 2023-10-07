use std::ops::Range;

use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct Hunk {
    /// 0-based index
    line_range: Range<usize>,
    lines: Vec<LineDiff>,
}
impl Hunk {
    pub fn get(old: &str, new: &str) -> Vec<Hunk> {
        let latest_committed_content = old;
        let current_content = new;

        let patch = diffy::DiffOptions::new()
            .set_context_len(0)
            .create_patch(latest_committed_content, current_content);
        let hunks = patch.hunks();

        hunks
            .iter()
            .filter_map(|hunk| {
                let line_range = hunk.new_range().range();
                let start = line_range.start.saturating_sub(1);
                let end = line_range.end.saturating_sub(1);
                let lines = hunk.lines();
                let inserted = lines
                    .iter()
                    .filter_map(|line| match line {
                        diffy::Line::Insert(inserted) => Some(inserted.to_string()),
                        _ => None,
                    })
                    .collect_vec()
                    .join("\n");
                let deleted = lines
                    .iter()
                    .filter_map(|line| match line {
                        diffy::Line::Delete(deleted) => Some(deleted.to_string()),
                        _ => None,
                    })
                    .collect_vec()
                    .join("\n");
                let _diff = diff::chars(&deleted, &inserted)
                    .into_iter()
                    .filter_map(|result| match result {
                        diff::Result::Left(left) => Some(left),
                        _ => None,
                    })
                    .collect::<String>();
                let lines = hunk.lines().iter().map(LineDiff::from);
                let min_leading_whitespaces_count = lines
                    .clone()
                    .map(|line| leading_whitespace_count(line.content()))
                    .min()
                    .unwrap_or_default();
                let trimmed = lines
                    .map(|line| line.trim_leading_whitespace(min_leading_whitespaces_count))
                    .collect_vec();

                // TODO: style the diff
                // - red for deleted line
                // - green for inserted line
                // - light red for deleted char within line
                // - light green for inserted char within line

                Some(Hunk {
                    line_range: start..end,
                    lines: trimmed,
                })
            })
            .collect_vec()
    }
    pub(crate) fn lines(&self) -> &Vec<LineDiff> {
        &self.lines
    }

    pub(crate) fn line_range(&self) -> &Range<usize> {
        &self.line_range
    }

    pub(crate) fn diff_strings(&self) -> Vec<String> {
        self.lines()
            .iter()
            .map(|line| {
                match line {
                    LineDiff::Context(context) => format!("  {}", context),
                    LineDiff::Delete(deleted) => format!("- {}", deleted),
                    LineDiff::Insert(inserted) => format!("+ {}", inserted),
                }
                .trim_end()
                .to_string()
            })
            .collect_vec()
    }

    pub fn to_string(&self) -> String {
        self.diff_strings().join("\n")
    }

    pub(crate) fn one_insert(message: &str) -> Hunk {
        Hunk {
            line_range: 0..0,
            lines: [LineDiff::Insert(message.to_string())].to_vec(),
        }
    }
}

fn leading_whitespace_count(s: &str) -> usize {
    s.chars().take_while(|c| c.is_whitespace()).count()
}

#[derive(Debug, Clone)]
pub enum LineDiff {
    Context(String),
    Delete(String),
    Insert(String),
}
impl LineDiff {
    fn content(&self) -> &str {
        use LineDiff::*;
        match self {
            Context(content) | Delete(content) | Insert(content) => content,
        }
    }

    fn trim_leading_whitespace(self, min_leading_whitespaces_count: usize) -> LineDiff {
        match self {
            LineDiff::Context(content) => {
                LineDiff::Context(trim_start(content, min_leading_whitespaces_count))
            }
            LineDiff::Delete(content) => {
                LineDiff::Delete(trim_start(content, min_leading_whitespaces_count))
            }
            LineDiff::Insert(content) => {
                LineDiff::Insert(trim_start(content, min_leading_whitespaces_count))
            }
        }
    }
}

fn trim_start(content: String, count: usize) -> String {
    content.chars().skip(count).collect()
}

impl From<&diffy::Line<'_, str>> for LineDiff {
    fn from(value: &diffy::Line<'_, str>) -> Self {
        match value {
            diffy::Line::Context(string) => LineDiff::Context(string.to_string()),
            diffy::Line::Delete(string) => LineDiff::Delete(string.to_string()),
            diffy::Line::Insert(string) => LineDiff::Insert(string.to_string()),
        }
    }
}

#[cfg(test)]
mod test_hunk {

    use indoc::indoc;
    use itertools::Itertools;

    use super::Hunk;

    #[test]
    fn should_trim_common_leading_whitespace() {
        let old = indoc!(
            "
          fn main() {
            let x = 3;
            fn nested() {
            }
          }
        "
        );
        let test = |new_content: &str, expected_indents: &[&[usize]]| {
            let hunks = Hunk::get(old, new_content);

            let actual_indents = hunks
                .into_iter()
                .map(|hunk| {
                    hunk.lines()
                        .iter()
                        .map(|line| {
                            line.content()
                                .chars()
                                .take_while(|c| c.is_whitespace())
                                .count()
                        })
                        .collect_vec()
                })
                .collect_vec();

            pretty_assertions::assert_eq!(expected_indents, actual_indents);
        };
        test(
            indoc!(
                "
          fn main() {
            let x = 3; // Changed this line
            fn nested() { // Changed this line
            }
          }
        "
            ),
            &[&[0, 0, 0, 0]],
        );

        test(
            indoc!(
                "
          fn main() { // Changed this line
            let x = 3;
            fn nested() { // Changed this line
            }
          }
        "
            ),
            &[&[0, 0], &[0, 0]],
        )
    }
}
