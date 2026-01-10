use crate::{buffer::Buffer, git::GitOperation};
use itertools::Itertools;

use super::{ByteRange, IterBasedSelectionMode};

pub struct GitHunk {
    ranges: Vec<super::ByteRange>,
}

impl GitHunk {
    pub fn new(
        diff_mode: &crate::git::DiffMode,
        buffer: &Buffer,
        working_directory: &shared::canonicalized_path::CanonicalizedPath,
    ) -> anyhow::Result<GitHunk> {
        let Some(path) = buffer.path() else {
            return Ok(GitHunk { ranges: Vec::new() });
        };
        let binding = path.file_diff(&buffer.content(), diff_mode, working_directory)?;
        let hunks = binding.hunks();
        let ranges = hunks
            .iter()
            .filter_map(|hunk| {
                let line_range = hunk.line_range();
                let byte_range = buffer.line_range_to_byte_range(line_range).ok()?;
                Some(ByteRange::new(byte_range).set_info(hunk.to_info()))
            })
            .collect_vec();
        Ok(GitHunk { ranges })
    }
}

impl IterBasedSelectionMode for GitHunk {
    fn iter<'a>(
        &'a self,
        _: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(self.ranges.clone().into_iter()))
    }
}
