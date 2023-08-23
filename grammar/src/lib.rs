pub mod grammar;

use etcetera::base_strategy::{choose_base_strategy, BaseStrategy};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

static CWD: RwLock<Option<PathBuf>> = RwLock::new(None);

static RUNTIME_DIRS: once_cell::sync::Lazy<Vec<PathBuf>> =
    once_cell::sync::Lazy::new(prioritize_runtime_dirs);

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

/// A list of runtime directories from highest to lowest priority
///
/// The priority is:
///
/// 1. sibling directory to `CARGO_MANIFEST_DIR` (if environment variable is set)
/// 2. subdirectory of user config directory (always included)
/// 3. `HELIX_RUNTIME` (if environment variable is set)
/// 4. subdirectory of path to helix executable (always included)
///
/// Postcondition: returns at least two paths (they might not exist).
fn prioritize_runtime_dirs() -> Vec<PathBuf> {
    const RT_DIR: &str = "runtime";
    // Adding higher priority first
    let mut rt_dirs = Vec::new();
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // this is the directory of the crate being run by cargo, we need the workspace path so we take the parent
        let path = PathBuf::from(dir).parent().unwrap().join(RT_DIR);
        log::debug!("runtime dir: {}", path.to_string_lossy());
        rt_dirs.push(path);
    }

    let conf_rt_dir = config_dir().join(RT_DIR);
    rt_dirs.push(conf_rt_dir);

    if let Ok(dir) = std::env::var("HELIX_RUNTIME") {
        rt_dirs.push(dir.into());
    }

    // fallback to location of the executable being run
    // canonicalize the path in case the executable is symlinked
    let exe_rt_dir = std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::canonicalize(path).ok())
        .and_then(|path| path.parent().map(|path| path.to_path_buf().join(RT_DIR)))
        .unwrap();
    rt_dirs.push(exe_rt_dir);

    rt_dirs
}

/// Runtime directories ordered from highest to lowest priority
///
/// All directories should be checked when looking for files.
///
/// Postcondition: returns at least one path (it might not exist).
pub fn runtime_dirs() -> &'static [PathBuf] {
    &RUNTIME_DIRS
}

/// Find file with path relative to runtime directory
///
/// `rel_path` should be the relative path from within the `runtime/` directory.
/// The valid runtime directories are searched in priority order and the first
/// file found to exist is returned, otherwise None.
fn find_runtime_file(rel_path: &Path) -> Option<PathBuf> {
    RUNTIME_DIRS.iter().find_map(|rt_dir| {
        let path = rt_dir.join(rel_path);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    })
}

/// Find file with path relative to runtime directory
///
/// `rel_path` should be the relative path from within the `runtime/` directory.
/// The valid runtime directories are searched in priority order and the first
/// file found to exist is returned, otherwise the path to the final attempt
/// that failed.
pub fn runtime_file(rel_path: &Path) -> PathBuf {
    find_runtime_file(rel_path).unwrap_or_else(|| {
        RUNTIME_DIRS
            .last()
            .map(|dir| dir.join(rel_path))
            .unwrap_or_default()
    })
}

pub fn config_dir() -> PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.config_dir();
    path.push("helix");
    path
}

pub fn cache_dir() -> PathBuf {
    // TODO: allow env var override
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.cache_dir();
    path.push("helix");
    path
}

pub fn config_file() -> PathBuf {
    CONFIG_FILE.get().map(|path| path.to_path_buf()).unwrap()
}

pub fn log_file() -> PathBuf {
    LOG_FILE.get().map(|path| path.to_path_buf()).unwrap()
}

pub fn workspace_config_file() -> PathBuf {
    find_workspace().0.join(".helix").join("config.toml")
}

pub fn lang_config_file() -> PathBuf {
    config_dir().join("languages.toml")
}

pub fn default_log_file() -> PathBuf {
    cache_dir().join("helix.log")
}

/// Finds the current workspace folder.
/// Used as a ceiling dir for LSP root resolution, the filepicker and potentially as a future filewatching root
///
/// This function starts searching the FS upward from the CWD
/// and returns the first directory that contains either `.git` or `.helix`.
/// If no workspace was found returns (CWD, true).
/// Otherwise (workspace, false) is returned
pub fn find_workspace() -> (PathBuf, bool) {
    let current_dir = current_working_dir();
    for ancestor in current_dir.ancestors() {
        if ancestor.join(".git").exists() || ancestor.join(".helix").exists() {
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
