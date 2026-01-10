use std::sync::mpsc::{channel, Receiver};

use crate::app::AppMessage;
use shared::canonicalized_path::CanonicalizedPath;

#[cfg(test)]
use crate::layout::BufferContentsMap;

pub struct TestRunner {
    temp_dir: CanonicalizedPath,
}

impl Drop for TestRunner {
    fn drop(&mut self) {
        self.temp_dir.remove_dir_all().unwrap();
    }
}

#[derive(Debug)]
pub struct TestOutput {
    pub term_output: Option<String>,
    pub buffer_contents_map: BufferContentsMap,
}

impl TestRunner {
    pub fn run(
        callback: impl Fn(CanonicalizedPath) -> anyhow::Result<TestOutput>,
    ) -> anyhow::Result<TestOutput> {
        let (runner, _) = Self::new()?;
        let output = callback(runner.temp_dir.clone())?;
        Ok(output)
    }
    fn new() -> anyhow::Result<(Self, Receiver<AppMessage>)> {
        const MOCK_REPO_PATH: &str = "mock_repos/rust1";

        let path = tempfile::tempdir()?.keep();
        std::fs::create_dir_all(path.clone())?;

        let options = fs_extra::dir::CopyOptions::new();
        fs_extra::dir::copy(MOCK_REPO_PATH, path.clone(), &options)?;

        let temp_dir = CanonicalizedPath::try_from(path)?.join("rust1")?;

        // Initialize the repo as a Git repo, so that we can test Git related features
        Self::git_init(temp_dir.clone())?;
        let (_, receiver) = channel();
        Ok((Self { temp_dir }, receiver))
    }
    fn git_init(path: CanonicalizedPath) -> anyhow::Result<()> {
        use git2::{Repository, RepositoryInitOptions};
        let repo = Repository::init_opts(path, RepositoryInitOptions::new().mkdir(false))?;
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;
        let sig = repo.signature()?;
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])?;
        Ok(())
    }
}
