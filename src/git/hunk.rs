use std::ops::Range;

use itertools::{Either, Itertools};

use crate::{
    components::suggestive_editor::{Decoration, Info},
    grid::StyleKey,
    selection_range::SelectionRange,
};

#[derive(Debug, Clone)]
pub struct Hunk {
    /// 0-based index
    line_range: Range<usize>,
    old: String,
    new: String,
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
                struct Line {
                    kind: LineKind,
                    content: String,
                }
                #[derive(PartialEq)]
                enum LineKind {
                    Delete,
                    Insert,
                }
                let (old, new): (Vec<_>, Vec<_>) = lines
                    .iter()
                    .filter_map(|line| match line {
                        diffy::Line::Context(_) => None,
                        diffy::Line::Delete(content) => Some(Line {
                            kind: LineKind::Delete,
                            content: content.to_string(),
                        }),
                        diffy::Line::Insert(content) => Some(Line {
                            kind: LineKind::Insert,
                            content: content.to_string(),
                        }),
                    })
                    .partition_map(|line| match line.kind {
                        LineKind::Delete => Either::Left(line.content),
                        LineKind::Insert => Either::Right(line.content),
                    });
                let old = old.join("");
                let new = new.join("");
                let min_leading_whitespaces_count = old
                    .lines()
                    .chain(new.lines())
                    .map(leading_whitespace_count)
                    .min()
                    .unwrap_or_default();
                let old = trim_start(old, min_leading_whitespaces_count);
                let new = trim_start(new, min_leading_whitespaces_count);

                // TODO: style the diff
                // - red for deleted line
                // - green for inserted line
                // - light red for deleted char within line
                // - light green for inserted char within line

                Some(Hunk {
                    line_range: start..end,
                    old,
                    new,
                })
            })
            .collect_vec()
    }
    pub(crate) fn line_range(&self) -> &Range<usize> {
        &self.line_range
    }

    pub(crate) fn one_insert(message: &str) -> Hunk {
        Hunk {
            line_range: 0..0,
            old: "".to_string(),
            new: message.to_string(),
        }
    }

    pub(crate) fn to_info(&self) -> Option<crate::components::suggestive_editor::Info> {
        let old_lines_len = self.old.lines().count();
        let content = self.old.clone() + "\n" + &self.new;
        let old_decorations = (0..old_lines_len).map(|line_index| {
            Decoration::new(SelectionRange::Line(line_index), StyleKey::HunkLineOld)
        });
        let new_lines_len = self.new.lines().count();
        let new_decorations = (0..new_lines_len).map(|line_index| {
            Decoration::new(
                SelectionRange::Line(line_index + old_lines_len),
                StyleKey::HunkLineNew,
            )
        });
        let char_diffs = if old_lines_len == 1 && new_lines_len == 1 {
            diff::chars(&self.old, &self.new)
                .into_iter()
                .enumerate()
                .filter_map(|(index, result)| {
                    let range = SelectionRange::Byte(index..index + 1);
                    match result {
                        diff::Result::Left(_) => {
                            Some(Decoration::new(range, StyleKey::HunkCharOld))
                        }
                        diff::Result::Both(_, _) => None,
                        diff::Result::Right(_) => None,
                    }
                })
                .collect_vec() // TODO: char diff for new char
        } else {
            Vec::new()
        };
        let decorations = old_decorations
            .chain(new_decorations)
            .chain(char_diffs)
            .collect_vec();
        Some(Info::new(content).set_decorations(decorations))
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
    content
        .lines()
        .map(|line| line.chars().skip(count).collect::<String>())
        .collect_vec()
        .join("\n")
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
                    (hunk.old + &hunk.new)
                        .lines()
                        .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
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
