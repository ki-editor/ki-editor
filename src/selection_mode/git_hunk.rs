use std::rc::Rc;

use crate::{buffer::Buffer, context::Context, git::GitOperation};
use itertools::Itertools;

use super::{
    get_current_selection_by_cursor_via_iter, ByteRange, PositionBasedSelectionMode, VectorBased,
    VectorBasedSelectionMode,
};

pub(crate) struct GitHunk {
    ranges: Rc<Vec<super::ByteRange>>,
}

impl GitHunk {
    pub(crate) fn new(
        diff_mode: &crate::git::DiffMode,
        buffer: &Buffer,
        context: &Context,
    ) -> anyhow::Result<GitHunk> {
        let Some(path) = buffer.path() else {
            return Ok(GitHunk {
                ranges: Rc::new(Vec::new()),
            });
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
        Ok(GitHunk {
            ranges: Rc::new(ranges),
        })
    }
}

impl VectorBasedSelectionMode for GitHunk {
    fn get_byte_ranges(&self, buffer: &Buffer) -> anyhow::Result<Rc<Vec<ByteRange>>> {
        Ok(self.ranges.clone())
    }
}
