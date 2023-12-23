use similar::{ChangeTag, TextDiff};
use std::ops::Range;

use itertools::Itertools;

use crate::{
    components::suggestive_editor::{Decoration, Info},
    grid::StyleKey,
    position::Position,
    selection_range::SelectionRange,
};

#[derive(Debug, Clone)]
pub struct Hunk {
    /// 0-based index
    new_line_range: Range<usize>,

    /// 0-based index
    old_line_range: Range<usize>,

    /// Used for displaying the diff.
    /// This field contains both the old content and the new content.
    content: String,
    new_content: String,
    decorations: Vec<Decoration>,
}
impl Hunk {
    pub fn get(old: &str, new: &str) -> Vec<Hunk> {
        let diff = TextDiff::from_lines(old, new);

        let context_len = 0;
        return diff
            .grouped_ops(context_len)
            .iter()
            .filter_map(|group| {
                // I'm going to assume each group only has one change (i.e. Delete/Insert/Replace)
                let (old_line_range, new_line_range) =
                    group.iter().find_map(|diff_op| match diff_op {
                        similar::DiffOp::Equal { .. } => None,
                        similar::DiffOp::Delete {
                            new_index,
                            old_index,
                            old_len,
                        } => Some((*old_index..(old_index + old_len), *new_index..*new_index)),
                        similar::DiffOp::Insert {
                            new_index,
                            new_len,
                            old_index,
                        } => Some((*old_index..*old_index, *new_index..(new_index + new_len))),
                        similar::DiffOp::Replace {
                            new_index,
                            new_len,
                            old_index,
                            old_len,
                        } => Some((
                            *old_index..(old_index + old_len),
                            *new_index..(new_index + new_len),
                        )),
                    })?;

                #[derive(PartialEq)]
                enum LineKind {
                    Delete,
                    Insert,
                }
                let (lines, decorations): (Vec<_>, Vec<_>) = group
                    .iter()
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
                                    .unzip();
                                let content = words.join("").to_string();
                                Some(((content, kind), decorations))
                            },
                        )
                    })
                    .unzip();
                let content = lines.iter().map(|(line, _)| line.trim_end()).join("\n");
                let new_content = lines
                    .iter()
                    .filter_map(|(line, kind)| match kind {
                        LineKind::Delete => None,
                        LineKind::Insert => Some(line.to_string()),
                    })
                    .collect_vec()
                    .join("");
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
                    new_line_range,
                    old_line_range,
                    content,
                    decorations,
                    new_content,
                })
            })
            .collect_vec();
    }
    pub(crate) fn line_range(&self) -> &Range<usize> {
        &self.new_line_range
    }

    pub(crate) fn one_insert(message: &str) -> Hunk {
        Hunk {
            new_line_range: 0..0,
            old_line_range: 0..0,
            content: message.to_string(),
            decorations: Vec::new(),
            new_content: "".to_string(),
        }
    }

    pub(crate) fn to_info(&self) -> Option<crate::components::suggestive_editor::Info> {
        let info = Info::new(self.content.clone()).set_decorations(self.decorations.clone());
        Some(info)
    }

    pub(crate) fn old_line_range(&self) -> Range<usize> {
        self.old_line_range.clone()
    }

    pub(crate) fn new_content(&self) -> String {
        self.new_content.to_string()
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
        buffer::Buffer, components::suggestive_editor::Decoration, grid::StyleKey,
        position::Position, selection_range::SelectionRange,
    };

    use super::Hunk;

    #[test]
    fn decorations() {
        // Note that both strings has leading spaces
        let hunks = Hunk::get("  Hello(world)", "  Hello(bumi)");
        assert_eq!(hunks.len(), 1);
        let actual = hunks[0].decorations.clone();
        // The hunk should trim the common leading spaces
        assert_eq!(hunks[0].content, "Hello(world)\nHello(bumi)");

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
                SelectionRange::Position(Position::new(0, 11)..Position::new(0, 12)),
                StyleKey::HunkOld,
            ),
            Decoration::new(
                SelectionRange::Position(Position::new(1, 0)..Position::new(1, 6)),
                StyleKey::HunkNew,
            ),
            Decoration::new(
                SelectionRange::Position(Position::new(1, 6)..Position::new(1, 10)),
                StyleKey::HunkNewEmphasized,
            ),
            Decoration::new(
                SelectionRange::Position(Position::new(1, 10)..Position::new(1, 11)),
                StyleKey::HunkNew,
            ),
        ]
        .to_vec();
        pretty_assertions::assert_eq!(actual, expected);

        // The inline diff should split at unicode grapheme boundary
        let buffer = Buffer::new(tree_sitter_md::language(), &hunks[0].content);
        let words = expected
            .into_iter()
            .flat_map(|decoraction| -> Result<ropey::Rope, anyhow::Error> {
                let range = decoraction.selection_range().to_char_index_range(&buffer)?;
                buffer.slice(&range)
            })
            .collect_vec();
        assert_eq!(words, vec!["Hello(", "world", ")", "Hello(", "bumi", ")"]);
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
