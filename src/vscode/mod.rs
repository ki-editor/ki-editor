//! VSCode integration module for Ki Editor
//!
//! This module contains the implementation of the VSCode integration for Ki Editor.
//! It handles communication with the VSCode extension host process, and provides
//! functionality for converting between Ki and VSCode data structures.

pub mod app;
pub mod handlers;
pub mod ipc;
pub mod logger;
pub mod utils;

use anyhow::Result;

/// Run Ki in VSCode integration mode
pub fn run_vscode() -> Result<()> {
    app::run_vscode()
}

// Expose the main VSCodeApp struct
pub use app::VSCodeApp;
