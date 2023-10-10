use similar::{ChangeTag, TextDiff};
use std::ops::Range;

use itertools::{Either, Itertools};

use crate::{
    components::suggestive_editor::{Decoration, Info},
    grid::StyleKey,
    position::Position,
    selection_range::SelectionRange,
};

#[derive(Debug, Clone)]
pub struct Hunk {
    /// 0-based index
    line_range: Range<usize>,
    content: String,
    decorations: Vec<Decoration>,
}
impl Hunk {
    pub fn get(old: &str, new: &str) -> Vec<Hunk> {
        let latest_committed_content = old;
        let current_content = new;

        let patch = diffy::DiffOptions::new()
            .set_context_len(0)
            .create_patch(latest_committed_content, current_content);
        let hunks = patch.hunks();
        let diff = TextDiff::from_lines(old, new);

        let context_len = 0;
        return diff
            .grouped_ops(context_len)
            .iter()
            .enumerate()
            .filter_map(|(idx, group)| {
                // I'm going to assume each group only has one change (i.e. Delete/Insert/Replace)
                let line_range = group.into_iter().find_map(|diff_op| match diff_op {
                    similar::DiffOp::Equal { .. } => None,
                    similar::DiffOp::Delete { new_index, .. } => Some(*new_index..*new_index),
                    similar::DiffOp::Insert {
                        new_index, new_len, ..
                    }
                    | similar::DiffOp::Replace {
                        new_index, new_len, ..
                    } => Some(*new_index..(new_index + new_len)),
                })?;
                struct Line {
                    kind: LineKind,
                    content: String,
                    decorations: Vec<Decoration>,
                }
                #[derive(PartialEq)]
                enum LineKind {
                    Delete,
                    Insert,
                }
                let (lines, decorations): (Vec<_>, Vec<_>) = group
                    .into_iter()
                    .flat_map(|diff_op| {
                        diff.iter_inline_changes(diff_op).enumerate().filter_map(
                            |(line_index, change)| {
                                let kind = match change.tag() {
                                    ChangeTag::Equal => None,
                                    ChangeTag::Delete => Some(LineKind::Delete),
                                    ChangeTag::Insert => Some(LineKind::Insert),
                                }?;
                                let (words, decorations): (Vec<_>, Vec<_>) = change
                                    .iter_strings_lossy()
                                    .scan(0, |column_index, (emphasized, value)| {
                                        let selection_range = SelectionRange::Position(
                                            Position::new(line_index, *column_index)
                                                ..Position::new(
                                                    line_index,
                                                    *column_index + value.len(),
                                                ),
                                        );
                                        *column_index += value.len();
                                        let style_key = match (&kind, emphasized) {
                                            (LineKind::Delete, true) => StyleKey::HunkOldEmphasized,
                                            (LineKind::Delete, false) => StyleKey::HunkOld,
                                            (LineKind::Insert, true) => StyleKey::HunkNewEmphasized,
                                            (LineKind::Insert, false) => StyleKey::HunkNew,
                                        };
                                        let decoration =
                                            Decoration::new(selection_range, style_key);
                                        Some((value.to_string(), decoration))
                                    })
                                    .into_iter()
                                    .unzip();
                                let content = words.join("").trim_end().to_string();
                                Some((content, decorations))
                            },
                        )
                    })
                    .unzip();
                let content = lines.join("\n");
                let min_leading_whitespaces_count = content
                    .lines()
                    .map(leading_whitespace_count)
                    .min()
                    .unwrap_or_default();
                let decorations = decorations
                    .into_iter()
                    .flatten()
                    .map(|decoration| decoration.move_left(min_leading_whitespaces_count))
                    .collect_vec();
                let content = trim_start(content, min_leading_whitespaces_count);
                Some(Hunk {
                    line_range,
                    content,
                    decorations,
                })
            })
            .collect_vec();
    }
    pub(crate) fn line_range(&self) -> &Range<usize> {
        &self.line_range
    }

    pub(crate) fn one_insert(message: &str) -> Hunk {
        Hunk {
            line_range: 0..0,
            content: message.to_string(),
            decorations: Vec::new(),
        }
    }

    pub(crate) fn to_info(&self) -> Option<crate::components::suggestive_editor::Info> {
        let info = Info::new(self.content.clone()).set_decorations(self.decorations.clone());
        Some(info)
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

    use crate::{
        components::suggestive_editor::Decoration, grid::StyleKey, position::Position,
        selection_range::SelectionRange,
    };

    use super::Hunk;

    #[test]
    fn decorations() {
        // Note that both strings has leading spaces
        let hunks = Hunk::get("  Hello world", "  Hello bumi");
        assert_eq!(hunks.len(), 1);
        let actual = hunks[0].decorations.clone();
        // The hunk should trim the common leading spaces
        assert_eq!(hunks[0].content, "Hello world\nHello bumi");
        let expected = [
            Decoration::new(
                SelectionRange::Position(Position::new(0, 0)..Position::new(0, 6)),
                StyleKey::HunkOld,
            ),
            Decoration::new(
                SelectionRange::Position(Position::new(0, 6)..Position::new(0, 11)),
                StyleKey::HunkOldEmphasized,
            ),
            Decoration::new(
                SelectionRange::Position(Position::new(1, 0)..Position::new(1, 6)),
                StyleKey::HunkNew,
            ),
            Decoration::new(
                SelectionRange::Position(Position::new(1, 6)..Position::new(1, 10)),
                StyleKey::HunkNewEmphasized,
            ),
        ]
        .to_vec();
        pretty_assertions::assert_eq!(actual, expected);
    }
    #[test]
    fn to_info_insertion() {
        let hunk = Hunk::get("a\nd", "a\nb\nc\nd")[0].clone();
        assert_eq!(hunk.content, "b\nc");
        assert_eq!(hunk.to_info().unwrap().content(), "b\nc")
    }

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
                    hunk.content
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
