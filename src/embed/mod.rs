//! Host integration module for Ki Editor
//!
//! This module contains the implementation of the Host integration for Ki Editor.
//! It handles communication with the Host extension host process, and provides
//! functionality for converting between Ki and Host data structures.

pub(crate) mod app;
pub(crate) mod handlers;
pub(crate) mod ipc;
pub(crate) mod logger;
pub(crate) mod utils;

pub(crate) use app::EmbeddedApp;
