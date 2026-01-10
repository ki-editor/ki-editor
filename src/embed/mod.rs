//! Host integration module for Ki Editor
//!
//! This module contains the implementation of the Host integration for Ki Editor.
//! It handles communication with the Host extension host process, and provides
//! functionality for converting between Ki and Host data structures.

pub mod app;
pub mod handlers;
pub mod ipc;
pub mod logger;
pub mod utils;

pub use app::EmbeddedApp;
