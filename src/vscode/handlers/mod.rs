//! Handlers for VSCode IPC messages
//!
//! This module contains handlers for various VSCode IPC messages, including
//! requests, notifications, and responses.

pub mod buffer;
pub mod cursor;
pub mod keyboard;
pub mod mode;
pub mod ping;
pub mod selection;
pub mod viewport;

/// Common imports and types for handlers
pub mod prelude {
    pub use anyhow::Result;
    pub use log::{error, info};

    // Use these types internally within the handlers module
    pub(crate) use crate::context::Context;
    pub(crate) use crate::position::Position;
    pub(crate) use crate::vscode::utils::*;
}
