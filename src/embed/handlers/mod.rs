//! Handlers for Host IPC messages
//!
//! This module contains handlers for various Host IPC messages, including
//! requests, notifications, and responses.

pub(crate) mod buffer;
pub(crate) mod keyboard;
pub(crate) mod ping;
pub(crate) mod selection;
pub(crate) mod viewport;

/// Common imports and types for handlers
pub(crate) mod prelude {
    pub use anyhow::Result;
    pub use log::{error, info};

    // Use these types internally within the handlers module
    pub(crate) use crate::context::Context;
    pub(crate) use crate::embed::utils::*;
    pub(crate) use crate::position::Position;
}
