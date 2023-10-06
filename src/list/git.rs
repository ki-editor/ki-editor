use rayon::prelude::*;
use std::{ops::Range, path::Path};

use git2::Repository;

use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

pub struct GitRepo {
    repo: Repository,
    path: CanonicalizedPath,
}

impl TryFrom<&CanonicalizedPath> for GitRepo {
    type Error = anyhow::Error;

    fn try_from(value: &CanonicalizedPath) -> Result<Self, Self::Error> {
        Ok(GitRepo {
            repo: Repository::open(value)?,
            path: value.clone(),
        })
    }
}

impl GitRepo {
    pub fn git_status_files(&self) -> Result<Vec<CanonicalizedPath>, anyhow::Error> {
        let statuses = self.repo.statuses(None)?;

        let new_and_modified_files: Vec<_> = statuses
            .into_iter()
            .filter(|entry| {
                let status = entry.status();
                status.is_wt_new() || status.is_wt_modified()
            })
            .filter_map(|entry| -> Option<CanonicalizedPath> {
                Some(entry.path()?.to_string().try_into().ok()?)
            })
            .collect();

        Ok(new_and_modified_files)
    }

    pub fn diffs(&self) -> anyhow::Result<Vec<FileDiff>> {
        let repo_path = self.path();
        Ok(self
            .git_status_files()?
            .into_iter()
            .par_bridge()
            .flat_map(|file| file.file_diff(&repo_path))
            .collect())
    }

    fn path(&self) -> &CanonicalizedPath {
        &self.path
    }
}

pub struct FileDiff {
    path: CanonicalizedPath,
    hunks: Vec<Hunk>,
}
impl FileDiff {
    pub(crate) fn hunks(&self) -> &Vec<Hunk> {
        &self.hunks
    }

    pub(crate) fn path(&self) -> &CanonicalizedPath {
        &self.path
    }
}

pub enum LineDiff {
    Context(String),
    Delete(String),
    Insert(String),
}

impl From<&diffy::Line<'_, str>> for LineDiff {
    fn from(value: &diffy::Line<'_, str>) -> Self {
        match value {
            diffy::Line::Context(string) => LineDiff::Context(string.to_string()),
            diffy::Line::Delete(string) => LineDiff::Delete(string.to_string()),
            diffy::Line::Insert(string) => LineDiff::Insert(string.to_string()),
        }
    }
}

pub struct Hunk {
    /// 0-based index
    line_range: Range<usize>,
    lines: Vec<LineDiff>,
}
impl Hunk {
    pub(crate) fn lines(&self) -> &Vec<LineDiff> {
        &self.lines
    }

    pub(crate) fn line_range(&self) -> &Range<usize> {
        &self.line_range
    }

    pub(crate) fn diff_strings(&self) -> Vec<String> {
        self.lines()
            .iter()
            .map(|line| {
                match line {
                    LineDiff::Context(context) => format!("  {}", context),
                    LineDiff::Delete(deleted) => format!("- {}", deleted),
                    LineDiff::Insert(inserted) => format!("+ {}", inserted),
                }
                .trim_end()
                .to_string()
            })
            .collect_vec()
    }
}
pub trait GitOperation {
    fn file_diff(&self, repo: &CanonicalizedPath) -> anyhow::Result<FileDiff>;
    fn content_at_last_commit(&self, repo: &GitRepo) -> anyhow::Result<String>;
}

impl GitOperation for CanonicalizedPath {
    fn file_diff(&self, repo_path: &CanonicalizedPath) -> anyhow::Result<FileDiff> {
        let latest_committed_content = self.content_at_last_commit(&repo_path.try_into()?)?;
        let current_content = self.read()?;

        let patch = diffy::DiffOptions::new()
            .set_context_len(0)
            .create_patch(&latest_committed_content, &current_content);
        let hunks = patch.hunks();

        Ok(FileDiff {
            path: self.clone(),
            hunks: hunks
                .into_iter()
                .filter_map(|hunk| {
                    let line_range = hunk.new_range().range();
                    let start = line_range.start.saturating_sub(1);
                    let end = line_range.end.saturating_sub(1);
                    let lines = hunk.lines();
                    let inserted = lines
                        .iter()
                        .filter_map(|line| match line {
                            diffy::Line::Insert(inserted) => Some(inserted.to_string()),
                            _ => None,
                        })
                        .collect_vec()
                        .join("\n");
                    let deleted = lines
                        .iter()
                        .filter_map(|line| match line {
                            diffy::Line::Delete(deleted) => Some(deleted.to_string()),
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

                    Some(Hunk {
                        line_range: start..end,
                        lines: hunk.lines().into_iter().map(From::from).collect_vec(),
                    })
                })
                .collect_vec(),
        })
    }

    fn content_at_last_commit(&self, repo: &GitRepo) -> anyhow::Result<String> {
        let head_commit = repo.repo.head()?.peel_to_commit()?;
        let tree = head_commit.tree()?;
        let entry = tree.get_path(std::path::Path::new(&self.display_relative()?))?;
        let blob = entry.to_object(&repo.repo)?.peel_to_blob()?;
        let content = blob.content().to_vec();
        Ok(String::from_utf8(content)?)
    }
}
