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
pub(crate) struct Hunk {
    /// 0-based index
    new_line_range: Range<usize>,

    /// Used for displaying the diff.
    /// This field contains both the old content and the new content.
    content: String,
    decorations: Vec<Decoration>,
}

#[derive(Debug, Clone)]
/// Simple Hunk is used for Git Gutter,
/// it is less expensive to compute as it needs less data
/// than `Hunk`.
pub(crate) struct SimpleHunk {
    /// 0-based index
    pub(crate) new_line_range: Range<usize>,
    pub(crate) new_content: String,
    pub(crate) old_content: String,

    pub(crate) kind: SimpleHunkKind,
}

#[derive(Debug, Clone)]
pub(crate) enum SimpleHunkKind {
    Delete,
    Insert,
    Replace,
}

impl Hunk {
    pub(crate) fn get_simple_hunks(old: &str, new: &str) -> Vec<SimpleHunk> {
        // We use imara_diff instead of `similar` because
        // imara_diff is much more faster, and more suitable
        // for computing git gutter.
        let input = imara_diff::InternedInput::new(old, new);
        let diff = imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input);
        diff.hunks()
            .map(|hunk| SimpleHunk {
                old_content: slice_line_range(old, &hunk.before),
                new_content: slice_line_range(new, &hunk.after),
                new_line_range: hunk.after.start as usize..hunk.after.end as usize,
                kind: if hunk.is_pure_insertion() {
                    SimpleHunkKind::Insert
                } else if hunk.is_pure_removal() {
                    SimpleHunkKind::Delete
                } else {
                    SimpleHunkKind::Replace
                },
            })
            .collect_vec()
    }

    pub(crate) fn get_hunks(old: &str, new: &str) -> Vec<Hunk> {
        let simple_hunks = Self::get_simple_hunks(old, new);
        simple_hunks
            .into_iter()
            .filter_map(|simple_hunk| {
                let old = &simple_hunk.old_content;
                let new = slice_line_range(
                    new,
                    &(simple_hunk.new_line_range.start as u32
                        ..simple_hunk.new_line_range.end as u32),
                );

                let (content, decorations) = Self::get_detailed_hunk(old, &new)?;
                Some(Hunk {
                    new_line_range: simple_hunk.new_line_range,
                    content,
                    decorations,
                })
            })
            .collect_vec()
    }

    pub(crate) fn get_detailed_hunk(old: &str, new: &str) -> Option<(String, Vec<Decoration>)> {
        let diff = TextDiff::from_lines(old, new);

        #[derive(PartialEq)]
        enum LineKind {
            Delete,
            Insert,
        }

        // We only take the first diff_op, because the `old` and `new` string should contain only one diff hunk.
        let diff_ops = diff.ops().iter().collect_vec();

        debug_assert_eq!(diff_ops.len(), 1);

        let diff_op = diff_ops.first()?;

        let (lines, decorations): (Vec<_>, Vec<_>) = diff
            .iter_inline_changes(diff_op)
            .enumerate()
            .filter_map(|(line_index, change)| {
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
                                ..Position::new(line_index, *column_index + value.len()),
                        );
                        *column_index += value.len();
                        let style_key = match (&kind, emphasized) {
                            (LineKind::Delete, true) => StyleKey::HunkOldEmphasized,
                            (LineKind::Delete, false) => StyleKey::HunkOld,
                            (LineKind::Insert, true) => StyleKey::HunkNewEmphasized,
                            (LineKind::Insert, false) => StyleKey::HunkNew,
                        };
                        let decoration = Decoration::new(selection_range, style_key);
                        Some((value.to_string(), decoration))
                    })
                    .unzip();
                let content = words.join("").to_string();
                Some(((content, kind), decorations))
            })
            .unzip();
        let content = lines.iter().map(|(line, _)| line.trim_end()).join("\n");
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
        Some((content, decorations))
    }

    pub(crate) fn line_range(&self) -> &Range<usize> {
        &self.new_line_range
    }

    pub(crate) fn one_insert(message: &str) -> Hunk {
        Hunk {
            new_line_range: 0..0,
            content: message.to_string(),
            decorations: Vec::new(),
        }
    }

    pub(crate) fn to_info(&self) -> Option<crate::components::suggestive_editor::Info> {
        let info = Info::new("Git Hunk Diff".to_string(), self.content.clone())
            .set_decorations(self.decorations.clone());
        Some(info)
    }
}

fn leading_whitespace_count(s: &str) -> usize {
    s.chars().take_while(|c| c.is_whitespace()).count()
}

#[derive(Debug, Clone)]
pub(crate) enum LineDiff {
    Context,
    Delete,
    Insert,
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
            diffy::Line::Context(_) => LineDiff::Context,
            diffy::Line::Delete(_) => LineDiff::Delete,
            diffy::Line::Insert(_) => LineDiff::Insert,
        }
    }
}

fn slice_line_range(content: &str, line_range: &Range<u32>) -> String {
    content
        .lines()
        .skip(line_range.start as usize)
        .take((line_range.end - line_range.start) as usize)
        .join("\n")
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
        let hunks = Hunk::get_hunks("  Hello(world)", "  Hello(bumi)");
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
        let buffer = Buffer::new(None, &hunks[0].content);
        let words = expected
            .into_iter()
            .flat_map(|decoration| -> Result<String, anyhow::Error> {
                let range = decoration.selection_range().to_char_index_range(&buffer)?;
                Ok(buffer.slice(&range)?.to_string())
            })
            .collect_vec();
        assert_eq!(words, vec!["Hello(", "world", ")", "Hello(", "bumi", ")"]);
    }
    #[test]
    fn to_info_insertion() {
        let hunk = Hunk::get_hunks("a\nd", "a\nb\nc\nd")[0].clone();
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
            let hunks = Hunk::get_hunks(old, new_content);

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

    #[test]
    fn simple_hunk_and_detailed_hunk_line_ranges_should_tally() {
        let old = indoc!(
            "
          foo
          bar
          spam
          baz
        "
        );
        let new = indoc!(
            "
          foo
          foo
          bar
          spam
          baz
          baz
        "
        );
        let simple_hunks_ranges = Hunk::get_simple_hunks(old, new)
            .into_iter()
            .map(|hunk| hunk.new_line_range)
            .collect_vec();

        let detail_hunks_ranges = Hunk::get_hunks(old, new)
            .into_iter()
            .map(|hunk| hunk.new_line_range)
            .collect_vec();

        assert_eq!(simple_hunks_ranges, detail_hunks_ranges)
    }
}
