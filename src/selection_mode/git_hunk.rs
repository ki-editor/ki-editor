use std::sync::{Arc, Mutex};

use git2::DiffOptions;

use crate::{buffer::Buffer, canonicalized_path::CanonicalizedPath};

use super::SelectionMode;

pub struct GitHunk {
    ranges: Vec<super::ByteRange>,
}

impl GitHunk {
    pub fn new(buffer: &Buffer) -> anyhow::Result<GitHunk> {
        let Some(buffer_path) =buffer.path() else {
            return Ok(GitHunk {
                ranges: Vec::new()
            })
        };
        let repo = git2::Repository::open(".")?;

        let mut delta_hunks = Arc::new(Mutex::new(Vec::new()));

        let delta_hunks_clone = delta_hunks.clone();
        // repo.diff_index_to_workdir(None, Some(&mut DiffOptions::new()))?.deltas().into_iter().map(|delta| {
        // });
        repo
            .diff_index_to_workdir(None, Some(&mut DiffOptions::new()))?
            .foreach(
                &mut |_, _| true,
                None,
                Some(&mut move |delta, hunk| {
                    let Some(path) = delta.new_file().path() else {
                        return true;
                    };
                    let Ok(path): Result<CanonicalizedPath, _> = path.try_into() else {
                        return true
                    };
                    if path == buffer_path {
                        log::info!("Found hunk for {:?}, {}, {}", hunk.header(), hunk.new_start(), hunk.new_lines());
                        let Ok( start) = buffer.line_to_byte(hunk.new_start() as usize) else {
                            return true;
                        };
                        let Ok(end) = buffer
                            .line_to_byte(hunk.new_start() as usize + hunk.new_lines() as usize) else {
                                return true
                        };

                        let end  = end.saturating_sub(1);
                        delta_hunks_clone.lock().unwrap().push(super::ByteRange::new(start..end));
                            true
                    } else {
                        true
                    }
                }),
                None,
            )?;

        let delta_hunks = delta_hunks.lock().unwrap();
        Ok(GitHunk {
            ranges: delta_hunks.to_vec(),
        })
    }
}

impl SelectionMode for GitHunk {
    fn iter<'a>(
        &'a self,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(self.ranges.clone().into_iter()))
    }
}
