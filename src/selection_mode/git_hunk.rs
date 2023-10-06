use crate::{
    buffer::Buffer,
    list::git::{GitOperation, LineDiff},
};
use itertools::Itertools;

use super::{ByteRange, SelectionMode};

pub struct GitHunk {
    ranges: Vec<super::ByteRange>,
}

impl GitHunk {
    pub fn new(buffer: &Buffer) -> anyhow::Result<GitHunk> {
        let Some(path) = buffer.path() else {
                return Ok(GitHunk {
                    ranges: Vec::new()
                });
            };
        let binding = path.file_diff(&".".try_into()?)?;
        let hunks = binding.hunks();
        let ranges = hunks
            .iter()
            .filter_map(|hunk| {
                let line_range = hunk.line_range();
                let lines = hunk.lines();
                let start = buffer
                    .line_to_byte(line_range.start.saturating_sub(1))
                    .ok()?;
                let end = buffer.line_to_byte(line_range.end.saturating_sub(1)).ok()?;
                let inserted = lines
                    .iter()
                    .filter_map(|line| match line {
                        LineDiff::Insert(inserted) => Some(inserted.to_string()),
                        _ => None,
                    })
                    .collect_vec()
                    .join("\n");
                let deleted = lines
                    .iter()
                    .filter_map(|line| match line {
                        LineDiff::Delete(deleted) => Some(deleted.to_string()),
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

                // TODO: style the diff
                // - red for deleted line
                // - green for inserted line
                // - light red for deleted char within line
                // - light green for inserted char within line

                Some(ByteRange::with_info(
                    start..end,
                    hunk.diff_strings().join("\n"),
                ))
            })
            .collect_vec();
        Ok(GitHunk { ranges })
    }
}

impl SelectionMode for GitHunk {
    fn name(&self) -> &'static str {
        "GIT HUNK"
    }
    fn iter<'a>(
        &'a self,
        _: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(self.ranges.clone().into_iter()))
    }
}
