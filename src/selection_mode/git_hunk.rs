use crate::{buffer::Buffer, git::GitOperation};
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
        // TODO: pass in current working directory
        let binding = path.file_diff(&".".try_into()?)?;
        let hunks = binding.hunks();
        let ranges = hunks
            .iter()
            .filter_map(|hunk| {
                let line_range = hunk.line_range();
                let start = buffer.line_to_byte(line_range.start).ok()?;
                let end = buffer.line_to_byte(line_range.end).ok()?;

                Some(ByteRange::with_info(start..end, hunk.to_string()))
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
