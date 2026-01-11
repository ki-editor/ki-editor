pub mod grammar;

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

static CWD: RwLock<Option<PathBuf>> = RwLock::new(None);

static RUNTIME_DIR: once_cell::sync::Lazy<PathBuf> = once_cell::sync::Lazy::new(get_runtime_dir);

static LOG_FILE: once_cell::sync::OnceCell<PathBuf> = once_cell::sync::OnceCell::new();

// Get the current working directory.
// This information is managed internally as the call to std::env::current_dir
// might fail if the cwd has been deleted.
pub fn current_working_dir() -> PathBuf {
    if let Some(path) = &*CWD.read().unwrap() {
        return path.clone();
    }

    let path = std::env::current_dir()
        .and_then(dunce::canonicalize)
        .expect("Couldn't determine current working directory");
    let mut cwd = CWD.write().unwrap();
    *cwd = Some(path.clone());

    path
}

fn get_runtime_dir() -> PathBuf {
    use directories::ProjectDirs;
    let project_dirs = ProjectDirs::from("ki", "ki", "ki").unwrap();
    std::fs::create_dir_all(project_dirs.config_dir()).unwrap();
    project_dirs.config_dir().into()
}

pub fn runtime_dir() -> &'static PathBuf {
    &RUNTIME_DIR
}

/// Find file with path relative to runtime directory
///
/// `rel_path` should be the relative path from within the `runtime/` directory.
/// The valid runtime directories are searched in priority order and the first
/// file found to exist is returned, otherwise None.
fn find_runtime_file(rel_path: &Path) -> PathBuf {
    RUNTIME_DIR.join(rel_path)
}

/// Find file with path relative to runtime directory
///
/// `rel_path` should be the relative path from within the `runtime/` directory.
/// The valid runtime directories are searched in priority order and the first
/// file found to exist is returned, otherwise the path to the final attempt
/// that failed.
pub fn runtime_file(rel_path: &Path) -> PathBuf {
    find_runtime_file(rel_path)
}

pub fn config_dir() -> PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.config_dir();
    path.push("ki");
    path
}

pub fn cache_dir() -> PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.cache_dir();
    path.push("ki");
    path
}

pub fn log_file() -> PathBuf {
    LOG_FILE.get().map(|path| path.to_path_buf()).unwrap()
}

pub fn default_log_file() -> PathBuf {
    cache_dir().join("ki.log")
}
