// TODO: use patch instead to get the diff
use std::path::Path;

use itertools::Itertools;

use crate::{buffer::Buffer, canonicalized_path::CanonicalizedPath, selection::Selection};

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
        let latest_committed_content = get_latest_file_content(".", &path)?;
        let current_content = buffer.rope().to_string();

        let patch = diffy::DiffOptions::new()
            .set_context_len(0)
            .create_patch(&latest_committed_content, &current_content);
        let hunks = patch.hunks();

        let ranges = hunks
            .into_iter()
            .filter_map(|hunk| {
                let line_range = hunk.new_range().range();
                let start = buffer
                    .line_to_byte(line_range.start.saturating_sub(1))
                    .ok()?;
                let end = buffer.line_to_byte(line_range.end.saturating_sub(1)).ok()?;
                Some(ByteRange::with_info(
                    start..end,
                    hunk.lines()
                        .into_iter()
                        .map(|line| match line {
                            diffy::Line::Context(context) => format!("  {}", context),
                            diffy::Line::Delete(deleted) => format!("- {}", deleted),
                            diffy::Line::Insert(inserted) => format!("+ {}", inserted),
                        })
                        .collect_vec()
                        .join(""),
                ))
            })
            .collect_vec();
        Ok(GitHunk { ranges })
    }
}

impl SelectionMode for GitHunk {
    fn iter<'a>(
        &'a self,
        _: &'a Selection,
        _: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(self.ranges.clone().into_iter()))
    }
}

fn get_latest_file_content(
    repo_path: &str,
    file_path: &CanonicalizedPath,
) -> anyhow::Result<String> {
    use git2::Repository;
    let repo = Repository::open(repo_path)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    let tree = head_commit.tree()?;
    let entry = tree.get_path(Path::new(&file_path.display_relative()?))?;
    let blob = entry.to_object(&repo)?.peel_to_blob()?;
    let content = blob.content().to_vec();
    Ok(String::from_utf8(content)?)
}
