use std::path::Path;

use git2::{BlameOptions, Repository};
use shared::canonicalized_path::CanonicalizedPath;

pub(crate) struct GitBlameInfo {
    commit: String,
    author: String,
    date: String,
    message: String,
    commit_url: anyhow::Result<String>,
}

impl GitBlameInfo {
    pub(crate) fn display(&self) -> String {
        format!(
            "Commit: {}
Author: {}
Date: {}
Message: {}
URL: {}
",
            self.commit,
            self.author,
            self.date,
            self.message,
            self.commit_url
                .as_ref()
                .map(|url| url.clone())
                .unwrap_or_else(|err| format!("[{err:?}]"))
        )
    }
}

/// `line_index` is 0-based.
pub(crate) fn blame_line(
    repo_path: &CanonicalizedPath,
    file_path: &CanonicalizedPath,
    line_index: usize,
) -> anyhow::Result<GitBlameInfo> {
    let repo = Repository::open(repo_path.to_path_buf())?;

    let line_number = line_index + 1;
    let mut blame_opts = BlameOptions::new();
    blame_opts.min_line(line_number);
    blame_opts.max_line(line_number);

    let relative_file_path = file_path.display_relative_to(repo_path)?;

    let blame = repo.blame_file(&Path::new(&relative_file_path), Some(&mut blame_opts))?;

    // Get the hunk for the specific line (lines are 1-indexed)
    let hunk = blame.get_line(line_number).ok_or_else(|| {
        anyhow::anyhow!("Unable to obtain blame for line number = {}", line_number)
    })?;

    let commit = repo.find_commit(hunk.final_commit_id())?;
    let commit_hash = hunk.final_commit_id().to_string();

    let git_time = commit.time();
    let timestamp = git_time.seconds();
    let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?
        .with_timezone(&chrono::Local);

    // Format the date nicely with system timezone
    let formatted_date = datetime.format("%Y-%m-%d %H:%M:%S %Z").to_string();

    // Get remote URL and format commit link
    let commit_url = repo
        .find_remote("origin")
        .or_else(|_| {
            repo.remotes()?
                .iter()
                .flatten()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No remotes found"))
                .and_then(|name| Ok(repo.find_remote(name)?))
        })
        .and_then(|remote| {
            remote
                .url()
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("URL is not valid utf-8."))
        })
        .and_then(|url| {
            format_commit_url(&url, &commit_hash, &relative_file_path, line_number)
                .ok_or_else(|| anyhow::anyhow!("Failed to normalize git url: {url:?}"))
        });

    Ok(GitBlameInfo {
        commit: commit_hash,
        author: format!("{}", commit.author()),
        date: formatted_date,
        message: commit
            .summary()
            .unwrap_or("[No commit message found]")
            .to_string(),
        commit_url,
    })
}

fn format_commit_url(
    remote_url: &str,
    commit_hash: &str,
    file_path: &str,
    line_number: usize,
) -> Option<String> {
    let (base_url, host_type) = normalize_git_url(remote_url)?;

    // Different platforms use different URL formats for line anchors
    match host_type {
        GitHost::GitHub => {
            // GitHub, Gitea, Gogs: /commit/{hash}#{file_path}L{line}
            Some(format!(
                "{}/blame/{}/{}#L{}",
                base_url, commit_hash, file_path, line_number
            ))
        }
    }
}

#[derive(Debug)]
enum GitHost {
    GitHub,
}

fn normalize_git_url(remote_url: &str) -> Option<(String, GitHost)> {
    let (base_url, host) = if let Some(ssh_url) = remote_url.strip_prefix("git@") {
        // Handle SSH URLs (git@host:user/repo.git)
        if let Some((host, path)) = ssh_url.split_once(':') {
            let clean_path = path.strip_suffix(".git").unwrap_or(path);
            (format!("https://{}/{}", host, clean_path), host.to_string())
        } else {
            return None;
        }
    } else if remote_url.starts_with("https://") || remote_url.starts_with("http://") {
        // Handle HTTPS URLs
        let clean_url = remote_url.strip_suffix(".git").unwrap_or(remote_url);
        let host = clean_url
            .strip_prefix("https://")
            .or_else(|| clean_url.strip_prefix("http://"))?
            .split('/')
            .next()?
            .to_string();
        (clean_url.to_string(), host)
    } else if let Some(path) = remote_url.strip_prefix("git://") {
        // Handle git:// protocol
        let clean_path = path.strip_suffix(".git").unwrap_or(path);
        let host = clean_path.split('/').next()?.to_string();
        (format!("https://{}", clean_path), host)
    } else if let Some(path) = remote_url.strip_prefix("ssh://git@") {
        // Handle SSH URLs with ssh:// prefix (ssh://git@host/user/repo.git)
        let clean_path = path.strip_suffix(".git").unwrap_or(path);
        let host = clean_path.split('/').next()?.to_string();
        (format!("https://{}", clean_path), host)
    } else {
        return None;
    };

    // Detect the Git hosting platform
    let git_host = if host.contains("github.com") {
        GitHost::GitHub
    } else {
        return None;
    };

    Some((base_url, git_host))
}
