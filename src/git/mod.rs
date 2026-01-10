pub mod blame;
pub mod hunk;

use anyhow::bail;
use rayon::prelude::*;

use git2::Repository;

use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use crate::git::hunk::SimpleHunk;

use self::hunk::Hunk;

pub struct GitRepo {
    repo: Repository,
    path: CanonicalizedPath,
}

impl TryFrom<&CanonicalizedPath> for GitRepo {
    type Error = anyhow::Error;

    fn try_from(value: &CanonicalizedPath) -> Result<Self, Self::Error> {
        let repo = Repository::discover(value)?;
        let path = match repo.path().parent() {
            Some(parent_path) => parent_path.try_into()?,
            None => bail!("cannot find parent path for {}", repo.path().display()),
        };

        Ok(GitRepo { repo, path })
    }
}

impl GitRepo {
    pub fn diffs(&self, diff_mode: DiffMode) -> anyhow::Result<Vec<FileDiff>> {
        Ok(self
            .diff_entries(diff_mode)?
            .into_iter()
            .par_bridge()
            .flat_map(|entry| entry.file_diff())
            .collect())
    }

    fn path(&self) -> &CanonicalizedPath {
        &self.path
    }

    pub fn diff_entries(&self, diff_mode: DiffMode) -> anyhow::Result<Vec<DiffEntry>> {
        // Open the repository
        let repo = &self.repo;

        let diff = {
            // Get the current branch
            let mut diff_options = DiffOptions::new();

            // Generate the diff
            diff_options.recurse_untracked_dirs(true);
            diff_options.include_untracked(true);

            let tree = self.get_tree(&diff_mode)?;
            repo.diff_tree_to_workdir(Some(&tree), Some(&mut diff_options))?
        };

        // Vector to hold the entries
        let entries = diff
            .deltas()
            // We will be conservative here, we will just ignore any errors.
            // We could have used `.map(...).collect::<Result<Vec<_>, _>>()` instead of
            // using `.flat_map(...), but that assumes nothing can ever goes wrong.
            // We don't want the user to not be able to get any diffs at all if some diffs
            // cannot be processed properly without error.
            .flat_map(|delta| -> anyhow::Result<_> {
                if !delta.new_file().exists() {
                    // It means this file is deleted
                    return Ok(None);
                }

                let new_path = delta
                    .new_file()
                    .path()
                    .ok_or(anyhow::anyhow!("No path found for delta.new_file()"))?
                    .to_str()
                    .ok_or(anyhow::anyhow!("Unable to convert path to string."))?
                    .to_string();

                let get_blob_content = |oid: git2::Oid| -> anyhow::Result<_> {
                    Ok(repo
                        .find_blob(oid)
                        .map(|blob| str::from_utf8(blob.content()).unwrap_or("").to_string())?)
                };
                // Get the old content
                let old_oid = delta.old_file().id();
                let old_content = get_blob_content(old_oid).ok();

                // Get the new content
                let new_oid = delta.new_file().id();
                let new_content = get_blob_content(new_oid).or_else(|_| -> anyhow::Result<_> {
                    Ok(std::fs::read_to_string(
                        repo.workdir()
                            .ok_or(anyhow::anyhow!(
                                "Unable to get repository working directory."
                            ))?
                            .join(new_path.clone()),
                    )?)
                })?;

                Ok(Some(DiffEntry {
                    new_path: self.path.join(&new_path)?,
                    old_content,
                    new_content,
                }))
            })
            .flatten()
            .collect_vec();

        Ok(entries)
    }

    fn get_tree(&self, diff_mode: &DiffMode) -> Result<git2::Tree<'_>, anyhow::Error> {
        match diff_mode {
            DiffMode::UnstagedAgainstMainBranch => Ok(self
                .repo
                .find_reference("refs/heads/main")
                .or_else(|_| self.repo.find_reference("refs/heads/master"))?
                .peel_to_commit()?
                .tree()?),
            DiffMode::UnstagedAgainstCurrentBranch => {
                Ok(self.repo.head()?.peel_to_commit()?.tree()?)
            }
        }
    }
}

pub struct FileDiff {
    path: CanonicalizedPath,
    hunks: Vec<Hunk>,
}
impl FileDiff {
    pub fn hunks(&self) -> &Vec<Hunk> {
        &self.hunks
    }

    pub fn path(&self) -> &CanonicalizedPath {
        &self.path
    }
}

pub trait GitOperation {
    fn file_diff(
        &self,
        current_content: &str,
        diff_mode: &DiffMode,
        repo: &CanonicalizedPath,
    ) -> anyhow::Result<FileDiff>;
    fn simple_hunks(
        &self,
        current_content: &str,
        diff_mode: &DiffMode,
        repo: &CanonicalizedPath,
    ) -> anyhow::Result<Vec<SimpleHunk>>;
    fn content_at_last_commit(
        &self,
        diff_mode: &DiffMode,
        repo: &GitRepo,
    ) -> anyhow::Result<String>;
}

impl GitOperation for CanonicalizedPath {
    fn file_diff(
        &self,
        current_content: &str,
        diff_mode: &DiffMode,
        repo_path: &CanonicalizedPath,
    ) -> anyhow::Result<FileDiff> {
        if let Ok(latest_committed_content) =
            self.content_at_last_commit(diff_mode, &repo_path.try_into()?)
        {
            let hunks = Hunk::get_hunks(&latest_committed_content, current_content);

            Ok(FileDiff {
                path: self.clone(),
                hunks,
            })
        } else {
            Ok(FileDiff {
                path: self.clone(),
                hunks: [Hunk::one_insert("[This file is untracked or renamed]")].to_vec(),
            })
        }
    }

    fn simple_hunks(
        &self,
        current_content: &str,
        diff_mode: &DiffMode,
        repo_path: &CanonicalizedPath,
    ) -> anyhow::Result<Vec<SimpleHunk>> {
        if let Ok(latest_committed_content) =
            self.content_at_last_commit(diff_mode, &repo_path.try_into()?)
        {
            let hunks = Hunk::get_simple_hunks(&latest_committed_content, current_content);

            Ok(hunks)
        } else {
            Ok(Default::default())
        }
    }

    fn content_at_last_commit(
        &self,
        diff_mode: &DiffMode,
        repo: &GitRepo,
    ) -> anyhow::Result<String> {
        let tree = repo.get_tree(diff_mode)?;
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
pub struct DiffEntry {
    new_path: CanonicalizedPath,
    old_content: Option<String>,
    new_content: String,
}

impl DiffEntry {
    fn file_diff(&self) -> anyhow::Result<FileDiff> {
        if let Some(old_content) = &self.old_content {
            let hunks = Hunk::get_hunks(old_content, &self.new_content);
            Ok(FileDiff {
                path: self.new_path.clone(),
                hunks,
            })
        } else {
            Ok(FileDiff {
                path: self.new_path.clone(),
                hunks: [Hunk::one_insert("[This file is untracked or renamed]")].to_vec(),
            })
        }
    }

    pub fn new_path(&self) -> CanonicalizedPath {
        self.new_path.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    UnstagedAgainstMainBranch,
    UnstagedAgainstCurrentBranch,
}

impl DiffMode {
    pub fn display(&self) -> String {
        match self {
            DiffMode::UnstagedAgainstMainBranch => "^".to_string(),
            DiffMode::UnstagedAgainstCurrentBranch => "@".to_string(),
        }
    }
}

#[cfg(test)]
mod test_git {
    use std::process::Command;
    use tempfile::tempdir;

    fn run_command(dir: &tempfile::TempDir, command: &str, args: &[&str]) {
        Command::new(command)
            .args(args)
            .current_dir(dir.path())
            .output()
            .expect("Failed to run command");
    }

    #[test]
    fn test_diff_entries() -> anyhow::Result<()> {
        let test = |mode: super::DiffMode, expected_old_content: &str| -> anyhow::Result<()> {
            // Create a temporary directory
            let dir = tempdir().unwrap();
            // Create files
            let file0 = dir.path().join("file0.txt");
            let file1 = dir.path().join("file1.txt");
            let file2 = dir.path().join("file2.txt");
            let file3 = dir.path().join("file3.txt");

            // Initialize a new repository
            run_command(&dir, "git", &["init"]);

            std::fs::write(file0.clone(), "hello\n")?;
            std::fs::write(file1.clone(), "hello\n")?;

            // First commit should contain two files
            run_command(&dir, "git", &["add", "."]);
            run_command(&dir, "git", &["commit", "-m", "First commit"]);

            // Checkout two a new branch
            run_command(&dir, "git", &["checkout", "-b", "new-branch"]);

            // Make two commits
            // Modify file1
            std::fs::write(file1.clone(), "hello\nworld")?;
            run_command(&dir, "git", &["add", "."]);
            run_command(&dir, "git", &["commit", "-m", "Second commit"]);

            std::fs::write(file1.clone(), "hello\nworld\nlook")?;
            run_command(&dir, "git", &["add", "."]);
            run_command(&dir, "git", &["commit", "-m", "Third commit"]);

            // Modify file1 again after commit
            std::fs::write(file1.clone(), "hello\nworld\nlook\nnow")?;

            // Create a new file named file2
            std::fs::write(file2.clone(), "This is file 2")?;

            // Create a new file named file3, and stage it
            std::fs::write(file3.clone(), "This is file 3")?;
            run_command(&dir, "git", &["add", file3.to_string_lossy().as_ref()]);

            // Create a new file at a new directory
            std::fs::create_dir_all(dir.path().join("organic"))?;
            let untracked_file_in_untracked_dir = dir.path().join("organic").join("nuts.txt");
            std::fs::write(untracked_file_in_untracked_dir.clone(), "This is nuts")?;
            // Deletes file0
            std::fs::remove_file(file0)?;

            // Check the diff
            let repo = super::GitRepo::try_from(&dir.path().try_into()?)?;
            let entries = repo.diff_entries(mode)?;
            let expected = [
                super::DiffEntry {
                    old_content: Some(expected_old_content.to_string()),
                    // Expect the new content is the latest content of the file
                    // regardless of whether it is commited/staged or not
                    new_content: "hello\nworld\nlook\nnow".to_string(),
                    new_path: file1.clone().try_into()?,
                },
                // Expect unstaged files (file 2) are included
                super::DiffEntry {
                    old_content: None,
                    new_content: "This is file 2".to_string(),
                    new_path: file2.clone().try_into()?,
                },
                // Expect staged files (file 3) are included
                super::DiffEntry {
                    old_content: None,
                    new_content: "This is file 3".to_string(),
                    new_path: file3.clone().try_into()?,
                },
                // Expect untracked files under an untracked directory are also included
                super::DiffEntry {
                    old_content: None,
                    new_content: "This is nuts".to_string(),
                    new_path: untracked_file_in_untracked_dir.clone().try_into()?,
                },
            ]
            .to_vec();
            assert_eq!(entries, expected);
            Ok(())
        };
        test(super::DiffMode::UnstagedAgainstMainBranch, "hello\n")?;
        test(
            super::DiffMode::UnstagedAgainstCurrentBranch,
            "hello\nworld\nlook",
        )?;
        Ok(())
    }
}
