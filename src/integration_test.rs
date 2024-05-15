use std::{
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
    time::UNIX_EPOCH,
};

use crate::app::AppMessage;
use shared::canonicalized_path::CanonicalizedPath;

pub struct TestRunner {
    temp_dir: CanonicalizedPath,
}

impl Drop for TestRunner {
    fn drop(&mut self) {
        self.temp_dir.remove_dir_all().unwrap();
    }
}
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn increment_counter() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

impl TestRunner {
    pub(crate) fn run(
        callback: impl Fn(CanonicalizedPath) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let (runner, _) = Self::new()?;
        callback(runner.temp_dir.clone())?;
        Ok(())
    }
    fn new() -> anyhow::Result<(Self, Receiver<AppMessage>)> {
        const MOCK_REPO_PATH: &str = "tests/mock_repos/rust1";

        // Copy the mock repo to a temporary directory using the current date
        // Why don't we use the `tempfile` crate? Because LSP doesn't work inside a the system temporary directory
        let epoch_time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let _random_number = rand::random::<u8>();
        let temp_dir = format!(
            "../temp_dir/{}_{}",
            epoch_time.as_secs(),
            increment_counter()
        );

        let path: PathBuf = temp_dir.into();
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
