//! Handlers for VSCode IPC messages
//!
//! This module contains handlers for various VSCode IPC messages, including
//! requests, notifications, and responses.

pub mod buffer;
pub mod cursor;
pub mod editor_action;
pub mod keyboard;
pub mod mode;
pub mod ping;
pub mod search;
pub mod selection;
pub mod selection_mode;
pub mod viewport;

/// Common imports and types for handlers
pub mod prelude {
    pub use anyhow::Result;
    pub use log::{debug, error, info, trace, warn};

    // Use these types internally within the handlers module
    pub(crate) use crate::context::Context;
    pub(crate) use crate::position::Position;
    pub(crate) use crate::vscode::utils::*;
}
