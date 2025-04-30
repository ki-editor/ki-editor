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
            anchor,
            active,
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
    // Added mode field to track the current selection mode
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mode: Option<SelectionMode>,
}

// Represents a single text edit operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct DiffEdit {
    pub range: Range,     // The range of the text to be replaced.
    pub new_text: String, // The new text to insert.
}

// Editor Mode enum for type-safe mode representation
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum EditorMode {
    Normal,
    Insert,
    MultiCursor,
    FindOneChar,
    Swap,
    Replace,
    Extend,
    // Add other modes as needed
}

// Selection Mode enum for type-safe selection mode representation
// This should match Ki's internal SelectionMode enum as closely as possible
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(tag = "type", content = "params")]
pub enum SelectionMode {
    Character,
    Line,
    LineFull,
    #[serde(rename = "word")]
    CoarseWord, // Word { skip_symbols: true }
    #[serde(rename = "fine_word")]
    FineWord, // Word { skip_symbols: false }
    Token,
    Custom,
    SyntaxNode,
    SyntaxNodeFine,
    Mark,
    // Simplified versions of complex modes
    Find,          // Find { search: Search }
    Diagnostic,    // Diagnostic(DiagnosticSeverityRange)
    GitHunk,       // GitHunk(DiffMode)
    LocalQuickfix, // LocalQuickfix { title: String }
}

// Editor actions enum for type-safe editor operations
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum EditorAction {
    Undo,
    Redo,
    Save,
    ForceSave,
    Copy,
    Cut,
    Paste,
    SelectAll,
    // Add other actions as needed to match DispatchEditor
}

// Implement Display for EditorAction for better logging and error messages
impl std::fmt::Display for EditorAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorAction::Undo => write!(f, "Undo"),
            EditorAction::Redo => write!(f, "Redo"),
            EditorAction::Save => write!(f, "Save"),
            EditorAction::ForceSave => write!(f, "ForceSave"),
            EditorAction::Copy => write!(f, "Copy"),
            EditorAction::Cut => write!(f, "Cut"),
            EditorAction::Paste => write!(f, "Paste"),
            EditorAction::SelectAll => write!(f, "SelectAll"),
        }
    }
}

// Message parameter structures

// Parameters for buffer events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BufferParams {
    pub uri: String,
    pub content: Option<String>,
    pub language_id: Option<String>,
    pub version: Option<i32>,
}

// Parameters for buffer diff events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct BufferDiffParams {
    pub buffer_id: String,
    pub edits: Vec<DiffEdit>, // A list of edits to apply sequentially.
}

// Parameters for cursor update events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CursorParams {
    pub buffer_id: String,
    pub anchors: Vec<Position>, // Anchor positions for multi-cursor
    pub actives: Vec<Position>, // Active/cursor positions for multi-cursor
}

// Parameters for mode change events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ModeParams {
    pub mode: String, // Using string for backward compatibility
    pub buffer_id: Option<String>,
}

// Parameters for typed mode change events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TypedModeParams {
    pub mode: EditorMode,
    pub buffer_id: Option<String>,
}

// Parameters for selection mode change events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SelectionModeParams {
    pub mode: SelectionMode,
    pub buffer_id: Option<String>,
}

// Parameters for viewport change events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ViewportParams {
    pub buffer_id: String,
    #[ts(type = "number")]
    pub start_line: usize,
    #[ts(type = "number")]
    pub end_line: usize,
}

// Parameters for keyboard input events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct KeyboardParams {
    pub key: String,
    #[ts(type = "number")]
    pub timestamp: u64,
    pub mode: Option<String>,
    pub is_composed: bool,
}

// Parameters for editor actions
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EditorActionParams {
    pub action: EditorAction,
    pub buffer_id: Option<String>,
}

// Parameters for external buffer events
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExternalBufferParams {
    pub buffer_id: String,
    pub content: String,
}

// Parameters for command execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CommandParams {
    pub name: String,
    pub args: Vec<String>,
    pub success: Option<bool>,
}

// Parameters for search operations
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

// Parameters for logging
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
    // System operations
    #[serde(rename = "ping")]
    Ping(Option<String>),

    // Buffer operations
    #[serde(rename = "buffer.open")]
    BufferOpen(BufferParams),
    #[serde(rename = "buffer.close")]
    BufferClose(BufferParams),
    #[serde(rename = "buffer.save")]
    BufferSave(BufferParams),
    #[serde(rename = "buffer.change")]
    BufferChange(BufferDiffParams),
    #[serde(rename = "buffer.active")]
    BufferActive(BufferParams),

    // Selection operations (includes cursor information)
    #[serde(rename = "selection.set")]
    SelectionSet(SelectionSet),

    // Mode operations
    #[serde(rename = "mode.set")]
    ModeSet(TypedModeParams),
    #[serde(rename = "selection_mode.set")]
    SelectionModeSet(SelectionModeParams),

    // Input operations
    #[serde(rename = "keyboard.input")]
    KeyboardInput(KeyboardParams),

    // Editor actions
    #[serde(rename = "editor.action")]
    EditorAction(EditorActionParams),

    // Search operations
    #[serde(rename = "search.find")]
    SearchFind(SearchParams),

    // Viewport operations
    #[serde(rename = "viewport.change")]
    ViewportChange(ViewportParams),
}

// Output Messages (Ki -> VSCode)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "tag", content = "params")]
pub enum OutputMessage {
    // System operations
    #[serde(rename = "ping")]
    Ping(String),
    #[serde(rename = "ki.log")]
    Log(LogParams),
    #[serde(rename = "error")]
    Error(String),
    #[serde(rename = "success")]
    Success(bool),

    // Buffer operations
    #[serde(rename = "buffer.open")]
    BufferOpen(BufferParams),
    #[serde(rename = "buffer.close")]
    BufferClose(BufferParams),
    #[serde(rename = "buffer.save")]
    BufferSave(BufferParams),
    #[serde(rename = "buffer.diff")]
    BufferDiff(BufferDiffParams),
    #[serde(rename = "buffer.activated")]
    BufferActivated(BufferParams),

    // Selection operations (includes cursor information)
    #[serde(rename = "selection.update")]
    SelectionUpdate(SelectionSet),

    // Mode operations
    #[serde(rename = "mode.change")]
    ModeChange(TypedModeParams),
    #[serde(rename = "selection_mode.change")]
    SelectionModeChange(SelectionModeParams),

    // Viewport operations
    #[serde(rename = "viewport.change")]
    ViewportChange(ViewportParams),

    // External buffer operations
    #[serde(rename = "external_buffer.created")]
    ExternalBufferCreated(ExternalBufferParams),
    #[serde(rename = "external_buffer.updated")]
    ExternalBufferUpdated(ExternalBufferParams),

    // Command operations
    #[serde(rename = "command.executed")]
    CommandExecuted(CommandParams),

    // Search operations
    #[serde(rename = "search.results")]
    SearchResults(String),

    // Editor actions
    #[serde(rename = "editor.action")]
    EditorAction(EditorActionParams),
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
    #[ts(optional)]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
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
            Self::SelectionSet(_) => Cow::Borrowed("selection.set"),
            Self::ModeSet(_) => Cow::Borrowed("mode.set"),
            Self::SelectionModeSet(_) => Cow::Borrowed("selection_mode.set"),
            Self::KeyboardInput(_) => Cow::Borrowed("keyboard.input"),
            Self::EditorAction(_) => Cow::Borrowed("editor.action"),
            Self::SearchFind(_) => Cow::Borrowed("search.find"),
            Self::ViewportChange(_) => Cow::Borrowed("viewport.change"),
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
            Self::SelectionSet(_) => "SelectionSet",
            Self::ModeSet(_) => "ModeSet",
            Self::SelectionModeSet(_) => "SelectionModeSet",
            Self::KeyboardInput(_) => "KeyboardInput",
            Self::EditorAction(_) => "EditorAction",
            Self::SearchFind(_) => "SearchFind",
            Self::ViewportChange(_) => "ViewportChange",
        }
    }
}

// Implementation for OutputMessage
impl MessageMethod for OutputMessage {
    fn method_name(&self) -> Cow<'static, str> {
        match self {
            Self::Ping(_) => Cow::Borrowed("ping"),
            Self::Log(_) => Cow::Borrowed("ki.log"),
            Self::Error(_) => Cow::Borrowed("error"),
            Self::Success(_) => Cow::Borrowed("success"),
            Self::BufferOpen(_) => Cow::Borrowed("buffer.open"),
            Self::BufferClose(_) => Cow::Borrowed("buffer.close"),
            Self::BufferSave(_) => Cow::Borrowed("buffer.save"),
            Self::BufferDiff(_) => Cow::Borrowed("buffer.diff"),
            Self::BufferActivated(_) => Cow::Borrowed("buffer.activated"),
            Self::SelectionUpdate(_) => Cow::Borrowed("selection.update"),
            Self::ModeChange(_) => Cow::Borrowed("mode.change"),
            Self::SelectionModeChange(_) => Cow::Borrowed("selection_mode.change"),
            Self::ViewportChange(_) => Cow::Borrowed("viewport.change"),
            Self::ExternalBufferCreated(_) => Cow::Borrowed("external_buffer.created"),
            Self::ExternalBufferUpdated(_) => Cow::Borrowed("external_buffer.updated"),
            Self::CommandExecuted(_) => Cow::Borrowed("command.executed"),
            Self::SearchResults(_) => Cow::Borrowed("search.results"),
            Self::EditorAction(_) => Cow::Borrowed("editor.action"),
        }
    }

    fn variant_name(&self) -> &'static str {
        match self {
            Self::Ping(_) => "Ping",
            Self::Log(_) => "Log",
            Self::Error(_) => "Error",
            Self::Success(_) => "Success",
            Self::BufferOpen(_) => "BufferOpen",
            Self::BufferClose(_) => "BufferClose",
            Self::BufferSave(_) => "BufferSave",
            Self::BufferDiff(_) => "BufferDiff",
            Self::BufferActivated(_) => "BufferActivated",
            Self::SelectionUpdate(_) => "SelectionUpdate",
            Self::ModeChange(_) => "ModeChange",
            Self::SelectionModeChange(_) => "SelectionModeChange",
            Self::ViewportChange(_) => "ViewportChange",
            Self::ExternalBufferCreated(_) => "ExternalBufferCreated",
            Self::ExternalBufferUpdated(_) => "ExternalBufferUpdated",
            Self::CommandExecuted(_) => "CommandExecuted",
            Self::SearchResults(_) => "SearchResults",
            Self::EditorAction(_) => "EditorAction",
        }
    }
}
