pub mod grammar;
use itertools::Itertools;

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

static CWD: RwLock<Option<PathBuf>> = RwLock::new(None);

static RUNTIME_DIR: once_cell::sync::Lazy<PathBuf> = once_cell::sync::Lazy::new(get_runtime_dir);

static CONFIG_FILE: once_cell::sync::OnceCell<PathBuf> = once_cell::sync::OnceCell::new();

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

pub fn set_current_working_dir(path: PathBuf) -> std::io::Result<()> {
    let path = dunce::canonicalize(path)?;
    std::env::set_current_dir(path.clone())?;
    let mut cwd = CWD.write().unwrap();
    *cwd = Some(path);
    Ok(())
}

pub fn initialize_config_file(specified_file: Option<PathBuf>) {
    let config_file = specified_file.unwrap_or_else(default_config_file);
    ensure_parent_dir(&config_file);
    CONFIG_FILE.set(config_file).ok();
}

pub fn initialize_log_file(specified_file: Option<PathBuf>) {
    let log_file = specified_file.unwrap_or_else(default_log_file);
    ensure_parent_dir(&log_file);
    LOG_FILE.set(log_file).ok();
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

pub fn config_file() -> PathBuf {
    CONFIG_FILE.get().map(|path| path.to_path_buf()).unwrap()
}

pub fn log_file() -> PathBuf {
    LOG_FILE.get().map(|path| path.to_path_buf()).unwrap()
}

pub fn workspace_config_file() -> PathBuf {
    find_workspace().0.join(".ki").join("config.toml")
}

pub fn lang_config_file() -> PathBuf {
    config_dir().join("languages.toml")
}

pub fn default_log_file() -> PathBuf {
    use chrono::Local;
    use std::process;

    let pid = process::id();
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M");

    log_dir().join(format!("ki_{}_{}.log", timestamp, pid))
}

pub fn log_dir() -> PathBuf {
    cache_dir().join("logs")
}

/// If `process_id` is not provided, the latest log file will be returned.
pub fn get_log_file(process_id: Option<String>) -> anyhow::Result<Option<PathBuf>> {
    let mut ls = std::fs::read_dir(log_dir())?
        .into_iter()
        .filter_map(|path| path.ok()?.file_name().into_string().ok());
    if let Some(process_id) = process_id {
        Ok(ls
            .find(|path| path.contains(&process_id))
            .map(|path| path.into()))
    } else {
        Ok(ls.sorted().last().map(|path| log_dir().join(path)))
    }
}

/// Finds the current workspace folder.
/// Used as a ceiling dir for LSP root resolution, the filepicker and potentially as a future filewatching root
///
/// This function starts searching the FS upward from the CWD
/// and returns the first directory that contains either `.git` or `.ki`.
/// If no workspace was found returns (CWD, true).
/// Otherwise (workspace, false) is returned
pub fn find_workspace() -> (PathBuf, bool) {
    let current_dir = current_working_dir();
    for ancestor in current_dir.ancestors() {
        if ancestor.join(".git").exists() || ancestor.join(".ki").exists() {
            return (ancestor.to_owned(), false);
        }
    }

    (current_dir, true)
}

fn default_config_file() -> PathBuf {
    config_dir().join("config.toml")
}

fn ensure_parent_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).ok();
        }
    }
}
