use std::rc::Rc;

use crate::{buffer::Buffer, context::Context, git::GitOperation};
use itertools::Itertools;

use super::{get_current_selection_by_cursor_via_iter, ByteRange, SelectionMode};

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

impl SelectionMode for GitHunk {
    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        get_current_selection_by_cursor_via_iter(
            buffer,
            cursor_char_index,
            if_current_not_found,
            self.ranges.clone(),
        )
    }
}
