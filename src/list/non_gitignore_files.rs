use anyhow::anyhow;
use itertools::Itertools;
use std::path::Path;

use crate::canonicalized_path::CanonicalizedPath;

pub fn non_git_ignored_files(directory: &CanonicalizedPath) -> anyhow::Result<Vec<String>> {
    use git2::{Repository, StatusOptions};

    let git_status_files = {
        let repo = Repository::open(&directory)?;
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.include_ignored(false);
        let statuses = repo.statuses(Some(&mut opts))?;
        statuses
            .iter()
            .filter(|entry| !entry.status().is_ignored())
            .filter_map(|entry| entry.path().map(|path| path.to_owned()))
            .filter_map(|path| {
                Some(
                    CanonicalizedPath::try_from(&path)
                        .ok()?
                        .display_relative()
                        .ok()?,
                )
            })
            .collect::<Vec<_>>()
    };

    let git_files = {
        let repo = git2::Repository::open(&directory)?;

        // Get the current branch name
        let head = repo.head()?.target().map(Ok).unwrap_or_else(|| {
            Err(anyhow!(
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
            let name = Path::new(root).join(entry_name);
            let name = name.to_string_lossy();
            result.push(name.to_string());
            git2::TreeWalkResult::Ok
        })?;

        result
    };
    Ok(git_files
        .into_iter()
        .chain(git_status_files)
        .unique_by(|item| item.clone())
        .collect_vec())
}
