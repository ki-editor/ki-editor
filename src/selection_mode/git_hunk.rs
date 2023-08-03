// TODO: use patch instead to get the diff
use std::sync::{Arc, Mutex};

use git2::{DiffLineType, DiffOptions};

use crate::{buffer::Buffer, canonicalized_path::CanonicalizedPath, selection::Selection};

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
        {
            // Create an empty vector to store the git signs
            let mut git_signs = Vec::new();
            // Get the head commit and its tree
            let head = repo.head().unwrap();
            let head_commit = head.peel_to_commit().unwrap();
            let head_tree = head_commit.tree().unwrap();

            // Get the index and its tree
            let mut index = repo.index().unwrap();
            let index_tree = index.write_tree().unwrap();

            // Create a diff options object
            let mut diff_opts = DiffOptions::new();
            // diff_opts.pathspec(buffer_path.display());

            // Get the diff between the head tree and the index tree
            let diff_head_index = repo
                .diff_tree_to_index(Some(&head_tree), Some(&index), Some(&mut diff_opts))
                .unwrap();

            // Get the diff between the index tree and the workdir
            let diff_index_workdir = repo
                .diff_index_to_workdir(Some(&index), Some(&mut diff_opts))
                .unwrap();

            // Iterate over the hunks and lines in the diff between the head tree and the index tree
            diff_index_workdir
                .foreach(
                    &mut |_, _| true, // file callback, do nothing
                    None,             // binary callback, do nothing
                    Some(&mut |_, hunk| {
                        // hunk callback, print hunk header
                        log::info!(
                            "header: {}",
                            String::from_utf8(hunk.header().to_vec()).unwrap().trim()
                        );
                        true
                    }),
                    Some(&mut |delta, _, line| {
                        let path: CanonicalizedPath =
                            delta.new_file().path().unwrap().try_into().unwrap();
                        if path != buffer_path {
                            return true;
                        }
                        // line callback, create git sign for each line
                        // Get the line number and change type from the line object
                        let line_number = line.new_lineno();
                        let change_type = line.origin();

                        let Some(line_number) = line.new_lineno() else {
                            return true;
                        };

                        // Create a git sign with the line number and change type
                        let git_sign = GitSign {
                            line_start: line_number as i32,
                            line_end: line_number as i32,
                            // change_type,
                        };
                        log::info!("gitsign: {:#?}", git_sign);

                        // Push the git sign to the vector
                        git_signs.push(git_sign);

                        // Print the line content with a prefix indicating the change type
                        log::info!(
                            "{} {}",
                            change_type as char,
                            String::from_utf8(line.content().to_vec()).unwrap()
                        );

                        true
                    }),
                )
                .unwrap();
        }
        return Ok(GitHunk { ranges: vec![] });

        let mut delta_hunks = Arc::new(Mutex::new(Vec::new()));

        let delta_hunks_clone = delta_hunks.clone();
        // repo.diff_index_to_workdir(None, Some(&mut DiffOptions::new()))?.deltas().into_iter().map(|delta| {
        // });
        repo.diff_index_to_workdir(None, Some(&mut DiffOptions::new()))?
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
                        log::info!(
                            "Found hunk for {:?}, old_start: {}, old_lines: {}, new_start: {}, new_lines: {}",
                            String::from_utf8(hunk.header().to_vec()).unwrap(),
                            hunk.old_start(),
                            hunk.old_lines(),
                            hunk.new_start(),
                            hunk.new_lines()
                        );
                        let start_line = hunk.new_start().saturating_sub(1) as usize;
                        let Ok(start) = buffer.line_to_byte(start_line) else {
                            return true;
                        };
                        let Ok(end) = buffer
                            .line_to_byte(start_line + hunk.new_lines() as usize) else {
                                return true
                        };

                        let end = end.saturating_sub(1);
                        delta_hunks_clone
                            .lock()
                            .unwrap()
                            .push(super::ByteRange::new(start..end));
                        true
                    } else {
                        true
                    }
                }),
                Some(&mut |_, _, diff_line| {
                    log::info!("diff_line: {:?}, content: {:?}, old_lineno = {:?}, new_lineno = {:?}, num_lines = {:?}",
                               diff_line.origin(), {
                        String::from_utf8(diff_line.content().to_vec()).unwrap()
                    },
                     diff_line.old_lineno(),
                     diff_line.new_lineno(),
                     diff_line.num_lines()


                    );
                    true
                }),
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
        current_selection: &'a Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(self.ranges.clone().into_iter()))
    }
}

#[derive(Debug)]
struct GitSign {
    line_start: i32,
    line_end: i32,
    // change_type: DiffLineType,
}
