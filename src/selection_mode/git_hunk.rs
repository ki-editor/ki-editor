use crate::{buffer::Buffer, context::Context, git::GitOperation};
use itertools::Itertools;

use super::{ByteRange, SelectionMode};

pub(crate) struct GitHunk {
    ranges: Vec<super::ByteRange>,
}

impl GitHunk {
    pub(crate) fn new(
        diff_mode: &crate::git::DiffMode,
        buffer: &Buffer,
        context: &Context,
    ) -> anyhow::Result<GitHunk> {
        let Some(path) = buffer.path() else {
            return Ok(GitHunk { ranges: Vec::new() });
        };
        let binding = path.file_diff(
            &buffer.content(),
            diff_mode,
            context.current_working_directory(),
        )?;
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
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
        Ok(self
            .ranges
            .iter()
            .find(|range| range.range.contains(&cursor_byte))
            .cloned())
    }
}
