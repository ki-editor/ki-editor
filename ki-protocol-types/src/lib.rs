// This crate defines the shared data structures for the Ki Editor VSCode IPC protocol.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use ts_rs::TS;

// Common data structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct Position {
    #[ts(type = "number")]
    pub line: usize,
    #[ts(type = "number")]
    pub character: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct Selection {
    pub anchor: Position, // The anchor (where selection started)
    pub active: Position, // The active/cursor position
    #[serde(default)]
    pub is_extended: bool,
}

impl Selection {
    pub fn new(anchor: Position, active: Position, is_extended: bool) -> Self {
        Selection {
            anchor: anchor.clone(),
            active: active.clone(),
            is_extended,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct SelectionSet {
    pub buffer_id: String,
    #[serde(default)]
    pub primary: usize,
    pub selections: Vec<Selection>,
}

// Message parameter structures
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BufferParams {
    pub uri: String,
    pub content: Option<String>,
    pub language_id: Option<String>,
    pub version: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BufferChange {
    pub buffer_id: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub version: i32,
    #[ts(type = "number")]
    pub message_id: u64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CursorParams {
    pub buffer_id: String,
    pub anchors: Vec<Position>, // Anchor positions for multi-cursor (optional, for future use)
    pub actives: Vec<Position>, // Active/cursor positions for multi-cursor
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ModeParams {
    pub mode: String,
    pub buffer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct KeyboardParams {
    pub key: String,
    #[ts(type = "number")]
    pub timestamp: u64,
    pub mode: Option<String>,
    pub is_composed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchParams {
    pub buffer_id: String,
    pub query: String,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub whole_word: bool,
    #[serde(default)]
    pub regex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CommandParams {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LogParams {
    pub level: String,
    pub message: String,
}

// Input Messages (VSCode -> Ki)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "tag", content = "params")]
pub enum InputMessage {
    // Buffer operations
    #[serde(rename = "buffer.open")]
    BufferOpen(BufferParams),
    #[serde(rename = "buffer.close")]
    BufferClose(BufferParams),
    #[serde(rename = "buffer.save")]
    BufferSave(BufferParams),
    #[serde(rename = "buffer.change")]
    BufferChange(BufferChange),
    #[serde(rename = "buffer.active")]
    BufferActive(BufferParams),

    // Cursor/Selection operations
    #[serde(rename = "cursor.update")]
    CursorUpdate(CursorParams),
    #[serde(rename = "cursor.get")]
    CursorGet,
    #[serde(rename = "selection.set")]
    SelectionSet(SelectionSet),
    #[serde(rename = "selection.get")]
    SelectionGet,

    // Mode operations
    #[serde(rename = "mode.set")]
    ModeSet(ModeParams),
    #[serde(rename = "selection_mode.set")]
    SelectionModeSet(ModeParams),

    // Input operations
    #[serde(rename = "keyboard.input")]
    KeyboardInput(KeyboardParams),

    // Search operations
    #[serde(rename = "search.find")]
    SearchFind(SearchParams),

    // System operations
    #[serde(rename = "ping")]
    Ping(Option<String>),
}

// Output Messages (Ki -> VSCode)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "tag", content = "params")]
pub enum OutputMessage {
    // Buffer operations
    #[serde(rename = "buffer.open")]
    BufferOpen(BufferParams),
    #[serde(rename = "buffer.close")]
    BufferClose(BufferParams),
    #[serde(rename = "buffer.save")]
    BufferSave(BufferParams),
    #[serde(rename = "buffer.change")]
    BufferChange(BufferChange),
    #[serde(rename = "buffer.update")]
    BufferUpdate(BufferChange),
    #[serde(rename = "buffer.diff")]
    BufferDiff(BufferChange),
    #[serde(rename = "buffer.ack")]
    BufferAck(u64), // message_id

    // Cursor/Selection operations
    #[serde(rename = "cursor.update")]
    CursorUpdate(CursorParams),
    #[serde(rename = "selection.update")]
    SelectionUpdate(SelectionSet),

    // Mode operations
    #[serde(rename = "mode.change")]
    ModeChange(ModeParams),
    #[serde(rename = "selection_mode.change")]
    SelectionModeChange(ModeParams),

    // Command operations
    #[serde(rename = "command")]
    Command(CommandParams),

    // System operations
    #[serde(rename = "ki.log")]
    Log(LogParams),
    #[serde(rename = "ping")]
    Ping(String),

    // Error handling
    #[serde(rename = "error")]
    Error(String),

    // Search operations
    #[serde(rename = "search.results")]
    SearchResults(String),

    // Success response
    #[serde(rename = "success")]
    Success(bool),
}

// Main message wrapper
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InputMessageWrapper {
    pub message: InputMessage,
    #[ts(type = "number")]
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OutputMessageWrapper {
    pub message: OutputMessage,
    #[ts(type = "number")]
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "any | null")]
    pub data: Option<serde_json::Value>,
}

// Define a more efficient MessageMethod trait with runtime and compile-time options
pub trait MessageMethod {
    // Dynamic method that gets the message name at runtime
    fn method_name(&self) -> Cow<'static, str>;

    // Static method for compile-time generated names (for optimized code paths)
    fn variant_name(&self) -> &'static str;
}

// Implementation for InputMessage
impl MessageMethod for InputMessage {
    fn method_name(&self) -> Cow<'static, str> {
        match self {
            Self::Ping(_) => Cow::Borrowed("ping"),
            Self::BufferOpen(_) => Cow::Borrowed("buffer.open"),
            Self::BufferClose(_) => Cow::Borrowed("buffer.close"),
            Self::BufferSave(_) => Cow::Borrowed("buffer.save"),
            Self::BufferChange(_) => Cow::Borrowed("buffer.change"),
            Self::BufferActive(_) => Cow::Borrowed("buffer.active"),
            Self::CursorUpdate(_) => Cow::Borrowed("cursor.update"),
            Self::CursorGet => Cow::Borrowed("cursor.get"),
            Self::SelectionSet(_) => Cow::Borrowed("selection.set"),
            Self::SelectionGet => Cow::Borrowed("selection.get"),
            Self::ModeSet(_) => Cow::Borrowed("mode.set"),
            Self::SelectionModeSet(_) => Cow::Borrowed("selection_mode.set"),
            Self::KeyboardInput(_) => Cow::Borrowed("keyboard.input"),
            Self::SearchFind(_) => Cow::Borrowed("search.find"),
        }
    }

    fn variant_name(&self) -> &'static str {
        match self {
            Self::Ping(_) => "Ping",
            Self::BufferOpen(_) => "BufferOpen",
            Self::BufferClose(_) => "BufferClose",
            Self::BufferSave(_) => "BufferSave",
            Self::BufferChange(_) => "BufferChange",
            Self::BufferActive(_) => "BufferActive",
            Self::CursorUpdate(_) => "CursorUpdate",
            Self::CursorGet => "CursorGet",
            Self::SelectionSet(_) => "SelectionSet",
            Self::SelectionGet => "SelectionGet",
            Self::ModeSet(_) => "ModeSet",
            Self::SelectionModeSet(_) => "SelectionModeSet",
            Self::KeyboardInput(_) => "KeyboardInput",
            Self::SearchFind(_) => "SearchFind",
        }
    }
}

// Implementation for OutputMessage
impl MessageMethod for OutputMessage {
    fn method_name(&self) -> Cow<'static, str> {
        match self {
            Self::Ping(_) => Cow::Borrowed("ping"),
            Self::BufferOpen(_) => Cow::Borrowed("buffer.open"),
            Self::BufferClose(_) => Cow::Borrowed("buffer.close"),
            Self::BufferSave(_) => Cow::Borrowed("buffer.save"),
            Self::BufferChange(_) => Cow::Borrowed("buffer.change"),
            Self::BufferUpdate(_) => Cow::Borrowed("buffer.update"),
            Self::BufferDiff(_) => Cow::Borrowed("buffer.diff"),
            Self::BufferAck(_) => Cow::Borrowed("buffer.ack"),
            Self::CursorUpdate(_) => Cow::Borrowed("cursor.update"),
            Self::SelectionUpdate(_) => Cow::Borrowed("selection.update"),
            Self::ModeChange(_) => Cow::Borrowed("mode.change"),
            Self::SelectionModeChange(_) => Cow::Borrowed("selection_mode.change"),
            Self::Command(_) => Cow::Borrowed("command"),
            Self::Log(_) => Cow::Borrowed("ki.log"),
            Self::Error(_) => Cow::Borrowed("error"),
            Self::SearchResults(_) => Cow::Borrowed("search.results"),
            Self::Success(_) => Cow::Borrowed("success"),
        }
    }

    fn variant_name(&self) -> &'static str {
        match self {
            Self::Ping(_) => "Ping",
            Self::BufferOpen(_) => "BufferOpen",
            Self::BufferClose(_) => "BufferClose",
            Self::BufferSave(_) => "BufferSave",
            Self::BufferChange(_) => "BufferChange",
            Self::BufferUpdate(_) => "BufferUpdate",
            Self::BufferDiff(_) => "BufferDiff",
            Self::BufferAck(_) => "BufferAck",
            Self::CursorUpdate(_) => "CursorUpdate",
            Self::SelectionUpdate(_) => "SelectionUpdate",
            Self::ModeChange(_) => "ModeChange",
            Self::SelectionModeChange(_) => "SelectionModeChange",
            Self::Command(_) => "Command",
            Self::Log(_) => "Log",
            Self::Error(_) => "Error",
            Self::SearchResults(_) => "SearchResults",
            Self::Success(_) => "Success",
        }
    }
}
