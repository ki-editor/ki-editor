use git2::Repository;

use shared::canonicalized_path::CanonicalizedPath;

pub fn git_status_files(path: &CanonicalizedPath) -> Result<Vec<String>, anyhow::Error> {
    let repo = Repository::open(path)?;
    let statuses = repo.statuses(None)?;

    let new_and_modified_files: Vec<_> = statuses
        .iter()
        .filter(|entry| {
            let status = entry.status();
            status.is_wt_new() || status.is_wt_modified()
        })
        .filter_map(|entry| Some(entry.path()?.to_string()))
        .collect();

    Ok(new_and_modified_files)
}
