use crate::{buffer::Buffer, git::GitOperation};
use itertools::Itertools;

use super::{ByteRange, SelectionMode};

pub(crate) struct GitHunk {
    ranges: Vec<super::ByteRange>,
}

impl GitHunk {
    pub(crate) fn new(
        diff_mode: &crate::git::DiffMode,
        buffer: &Buffer,
    ) -> anyhow::Result<GitHunk> {
        let Some(path) = buffer.path() else {
            return Ok(GitHunk { ranges: Vec::new() });
        };
        // TODO: pass in current working directory
        let binding = path.file_diff(diff_mode, &".".try_into()?)?;
        let hunks = binding.hunks();
        let ranges = hunks
            .iter()
            .filter_map(|hunk| {
                let line_range = hunk.line_range();
                let start = buffer.line_to_byte(line_range.start).ok()?;
                let end = buffer.line_to_byte(line_range.end).ok()?;

                Some(ByteRange::new(start..end).set_info(hunk.to_info()))
            })
            .collect_vec();
        Ok(GitHunk { ranges })
    }
}

impl SelectionMode for GitHunk {
    fn iter<'a>(
        &'a self,
        _: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(self.ranges.clone().into_iter()))
    }
}
