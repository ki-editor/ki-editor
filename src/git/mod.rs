pub(crate) mod hunk;

use rayon::prelude::*;

use git2::Repository;

use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use self::hunk::Hunk;

pub(crate) struct GitRepo {
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
    pub(crate) fn git_status_files(&self) -> Result<Vec<CanonicalizedPath>, anyhow::Error> {
        let entries = diff_entries(&self.path, DiffMode::UnstagedAgainstCurrentBranch)?;
        return Ok(entries
            .into_iter()
            .map(|entry| entry.new_path)
            .collect_vec());
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

    pub(crate) fn diffs(&self) -> anyhow::Result<Vec<FileDiff>> {
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

    pub(crate) fn non_git_ignored_files(&self) -> anyhow::Result<Vec<CanonicalizedPath>> {
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
            .filter(|path| path.is_file())
            .unique_by(|item| item.clone())
            .collect_vec())
    }
}

pub(crate) struct FileDiff {
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
use git2::DiffOptions;

use std::str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiffEntry {
    new_path: CanonicalizedPath,
    old_content: String,
    new_content: String,
}

#[derive(Debug)]
enum DiffMode {
    CurrentBranchAgainstDefaultBranch,
    UnstagedAgainstCurrentBranch,
}

fn diff_entries(
    working_directory: &CanonicalizedPath,
    mode: DiffMode,
) -> anyhow::Result<Vec<DiffEntry>> {
    println!("diff_entries working_directory = {working_directory:?}");
    println!("diff_entries mode = {mode:?}");
    // Open the repository
    let repo = Repository::open(working_directory)?;

    // Get the current branch
    let head = repo.head()?;
    let current_branch = head.peel_to_commit()?;
    // Get the default branch (usually 'main' or 'master')
    let default_branch = repo
        .find_reference("refs/heads/main")
        .or_else(|_| repo.find_reference("refs/heads/master"))?
        .peel_to_commit()?;

    // Create a DiffOptions
    let mut diff_options = DiffOptions::new();

    // Generate the diff
    let diff = match mode {
        DiffMode::CurrentBranchAgainstDefaultBranch => repo.diff_tree_to_tree(
            Some(&default_branch.tree()?),
            Some(&current_branch.tree()?),
            Some(&mut diff_options),
        )?,
        DiffMode::UnstagedAgainstCurrentBranch => {
            diff_options.include_untracked(true);
            repo.diff_tree_to_workdir(Some(&current_branch.tree()?), Some(&mut diff_options))?
        }
    };

    println!("diff = {:?}", diff.deltas().count());

    // Vector to hold the entries
    let entries = diff
        .deltas()
        .into_iter()
        .map(|delta| -> anyhow::Result<_> {
            let new_path = delta
                .new_file()
                .path()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            println!("new_path = {new_path:?}");

            let get_blob_content = |oid: git2::Oid| -> anyhow::Result<_> {
                // The oid is zero (nullish) when the it represents an unstaged file
                if oid.is_zero() {
                    return Ok(std::fs::read_to_string(
                        repo.workdir()
                            .ok_or(anyhow::anyhow!(
                                "Unable to get repository working directory."
                            ))?
                            .join(new_path.clone()),
                    )?);
                } else {
                    repo.find_blob(oid)
                        .and_then(|blob| {
                            Ok(str::from_utf8(blob.content()).unwrap_or("").to_string())
                        })
                        .or_else(|_| {
                            Ok(std::fs::read_to_string(
                                repo.workdir()
                                    .ok_or(anyhow::anyhow!(
                                        "Unable to get repository working directory."
                                    ))?
                                    .join(new_path.clone()),
                            )?)
                        })
                }
            };

            // Get the old content
            let old_oid = delta.old_file().id();
            let old_content = get_blob_content(old_oid)?;

            // Get the new content
            let new_oid = delta.new_file().id();
            let new_content = get_blob_content(new_oid)?;

            Ok(DiffEntry {
                new_path: working_directory.join(&new_path)?,
                old_content,
                new_content,
            })
        })
        .collect::<anyhow::Result<Vec<_>, _>>()?;

    // Iterate over each diff
    println!("entries = {entries:#?}");
    Ok(entries)
}

#[cfg(test)]
mod test_git {
    use rand::Rng;
    use std::process::Command;
    use tempfile::tempdir;

    use super::diff_entries;

    fn run_command(dir: &tempfile::TempDir, command: &str, args: &[&str]) {
        Command::new(command)
            .args(args)
            .current_dir(dir.path())
            .output()
            .expect("Failed to run command");
    }

    #[test]
    fn test_diff_entries() -> anyhow::Result<()> {
        // Create a temporary directory
        let dir = tempdir().unwrap();
        // Create file
        let file1 = dir.path().join("file1.txt");
        let test = |mode: super::DiffMode,
                    expected_old_content: &str,
                    expected_new_content: &str|
         -> anyhow::Result<()> {
            // Initialize a new repository
            run_command(&dir, "git", &["init"]);

            std::fs::write(file1.clone(), "hello\n")?;

            run_command(&dir, "git", &["add", "."]);
            run_command(&dir, "git", &["commit", "-m", "First commit"]);

            // Create a new branch
            run_command(&dir, "git", &["checkout", "-b", "new-branch"]);

            // Make two commits
            // Modify file1
            std::fs::write(file1.clone(), "hello\nworld")?;
            run_command(&dir, "git", &["add", "."]);
            run_command(&dir, "git", &["commit", "-m", "Second commit"]);

            std::fs::write(file1.clone(), "hello\nworld\nlook")?;
            run_command(&dir, "git", &["add", "."]);
            run_command(&dir, "git", &["commit", "-m", "Third commit"]);

            std::fs::write(file1.clone(), "hello\nworld\nlook\nnow")?;

            // Check the diff
            let entries = diff_entries(&dir.path().to_path_buf().try_into()?, mode)
                .expect("Failed to get diff");
            assert_eq!(
                entries,
                vec![super::DiffEntry {
                    new_content: expected_new_content.to_string(),
                    old_content: expected_old_content.to_string(),
                    new_path: file1.clone().try_into()?,
                }]
            );
            Ok(())
        };
        test(
            super::DiffMode::CurrentBranchAgainstDefaultBranch,
            "hello\n",
            "hello\nworld\nlook",
        )?;
        test(
            super::DiffMode::UnstagedAgainstCurrentBranch,
            "hello\nworld\nlook",
            "hello\nworld\nlook\nnow",
        )?;
        Ok(())
    }
}
