pub mod hunk;

use rayon::prelude::*;

use git2::Repository;

use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use self::hunk::Hunk;

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
                let path = self.path.join(entry.path()?).ok()?;
                Some(path)
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
            .flat_map(|file| file.file_diff(repo_path))
            .collect())
    }

    fn path(&self) -> &CanonicalizedPath {
        &self.path
    }

    pub fn non_git_ignored_files(&self) -> anyhow::Result<Vec<CanonicalizedPath>> {
        let git_status_files = self.git_status_files()?;

        let git_files = {
            let repo = git2::Repository::open(&self.path)?;

            // Get the current branch name
            let head = repo.head()?.target().map(Ok).unwrap_or_else(|| {
                Err(anyhow::anyhow!(
                    "Couldn't find HEAD for repository {}",
                    repo.path().display(),
                ))
            })?;

            // Get the generic object of the current branch
            let object = repo.find_object(head, None)?;

            // Get the tree object of the current branch
            let tree = object.peel_to_tree()?;

            let mut result = vec![];
            // Iterate over the tree entries and print their names
            tree.walk(git2::TreeWalkMode::PostOrder, |root, entry| {
                let entry_name = entry.name().unwrap_or_default();
                if let Ok(path) = self.path.join(
                    &std::path::Path::new(root)
                        .join(entry_name)
                        .to_string_lossy(),
                ) {
                    result.push(path)
                };
                git2::TreeWalkResult::Ok
            })?;

            result
        };
        Ok(git_files
            .into_iter()
            .chain(git_status_files)
            .filter_map(|path| -> Option<CanonicalizedPath> { path.try_into().ok() })
            .filter(|path| path.is_file())
            .unique_by(|item| item.clone())
            .collect_vec())
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

pub trait GitOperation {
    fn file_diff(&self, repo: &CanonicalizedPath) -> anyhow::Result<FileDiff>;
    fn content_at_last_commit(&self, repo: &GitRepo) -> anyhow::Result<String>;
}

impl GitOperation for CanonicalizedPath {
    fn file_diff(&self, repo_path: &CanonicalizedPath) -> anyhow::Result<FileDiff> {
        if let Ok(latest_committed_content) = self.content_at_last_commit(&repo_path.try_into()?) {
            let current_content = self.read()?;
            let hunks = Hunk::get(&latest_committed_content, &current_content);

            Ok(FileDiff {
                path: self.clone(),
                hunks,
            })
        } else {
            Ok(FileDiff {
                path: self.clone(),
                hunks: [Hunk::one_insert("[This file is untracked by Git]")].to_vec(),
            })
        }
    }

    fn content_at_last_commit(&self, repo: &GitRepo) -> anyhow::Result<String> {
        let head_commit = repo.repo.head()?.peel_to_commit()?;
        let tree = head_commit.tree()?;
        let entry = tree.get_path(std::path::Path::new(
            &self.display_relative_to(repo.path())?,
        ))?;
        let blob = entry.to_object(&repo.repo)?.peel_to_blob()?;
        let content = blob.content().to_vec();
        Ok(String::from_utf8(content)?)
    }
}
