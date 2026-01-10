//! Handlers for Host IPC messages
//!
//! This module contains handlers for various Host IPC messages, including
//! requests, notifications, and responses.

pub mod buffer;
pub mod keyboard;
pub mod ping;
pub mod selection;
pub mod viewport;

/// Common imports and types for handlers
pub mod prelude {
    pub use anyhow::Result;
    pub use log::{error, info};

    // Use these types internally within the handlers module
    pub use crate::embed::utils::*;
    pub use crate::position::Position;
}
