///! This crate defines the shared data structures for the Ki Editor and Host application IPC protocol.
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use typeshare::typeshare;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[typeshare]
pub struct Position {
    #[typeshare(typescript(type = "number"))]
    pub line: u32,
    #[typeshare(typescript(type = "number"))]
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[typeshare]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[typeshare]
pub struct Selection {
    /// The other end of the selection
    pub anchor: Position,
    /// The cursor position
    pub active: Position,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[typeshare]
pub struct SelectionSet {
    pub buffer_id: String,
    #[serde(default)]
    #[typeshare(typescript(type = "number"))]
    pub primary: u32,
    pub selections: Vec<Selection>,
}

/// Represents a single text edit operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[typeshare]
pub struct DiffEdit {
    /// The range of the text to be replaced.
    pub range: Range,
    /// The new text to replace the range.
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[typeshare]
#[serde(rename_all = "camelCase")]
pub enum EditorMode {
    Normal,
    Insert,
    MultiCursor,
    FindOneChar,
    Swap,
    Replace,
    Extend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[typeshare]
#[serde(tag = "type", content = "params")]
pub enum SelectionMode {
    Character,
    Line,
    LineFull,
    Word,
    WordFine,
    Token,
    Custom,
    SyntaxNode,
    SyntaxNodeFine,
    Mark,
    // Simplified versions of complex modes
    Find { search: String },
    Diagnostic(DiagnosticKind),
    GitHunk,
    LocalQuickfix,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[typeshare]
pub enum DiagnosticKind {
    Error,
    Information,
    Warning,
    All,
    Hint,
}

// Editor actions enum for type-safe editor operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[typeshare]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct BufferParams {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct BufferContentParams {
    pub uri: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct BufferOpenParams {
    pub uri: String,
    pub selections: Vec<Selection>,
    pub content: String,
}

// Parameters for buffer diff events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[typeshare]
pub struct BufferDiffParams {
    pub buffer_id: String,
    pub edits: Vec<DiffEdit>, // A list of edits to apply sequentially.
}

// Parameters for cursor update events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct CursorParams {
    pub buffer_id: String,
    pub anchors: Vec<Position>, // Anchor positions for multi-cursor
    pub actives: Vec<Position>, // Active/cursor positions for multi-cursor
}

// Parameters for mode change events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct ModeParams {
    pub mode: String, // Using string for backward compatibility
    pub buffer_id: Option<String>,
}

// Parameters for typed mode change events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct TypedModeParams {
    pub mode: EditorMode,
    pub buffer_id: Option<String>,
}

// Parameters for selection mode change events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct SelectionModeParams {
    pub mode: SelectionMode,
    pub buffer_id: Option<String>,
}

// Parameters for viewport change events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct LineRange {
    #[typeshare(typescript(type = "number"))]
    pub start: u32,
    #[typeshare(typescript(type = "number"))]
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct ViewportParams {
    pub buffer_id: String,
    pub visible_line_ranges: Vec<LineRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[typeshare]
pub struct KeyboardParams {
    pub key: String,
    pub uri: String,
    /// This is necessary for resolving buffer content desync
    /// between Ki and the host application
    pub content_hash: u32,
}

// Parameters for external buffer events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct ExternalBufferParams {
    pub buffer_id: String,
    pub content: String,
}

// Parameters for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct CommandParams {
    pub name: String,
    pub args: Vec<String>,
    pub success: Option<bool>,
}

// Parameters for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
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

// Input Messages (Host -> Ki)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
#[serde(tag = "tag", content = "params")]
pub enum InputMessage {
    // System operations
    #[serde(rename = "ping")]
    Ping(Option<String>),

    // Buffer operations
    #[serde(rename = "buffer.open")]
    BufferOpen(BufferOpenParams),
    #[serde(rename = "buffer.change")]
    BufferChange(BufferDiffParams),
    #[serde(rename = "buffer.active")]
    BufferActive(BufferParams),
    #[serde(rename = "editor.syncBufferResponse")]
    SyncBufferResponse(BufferContentParams),

    // Selection operations (includes cursor information)
    #[serde(rename = "selection.set")]
    SelectionSet(SelectionSet),

    // Input operations
    #[serde(rename = "keyboard.input")]
    KeyboardInput(KeyboardParams),

    // Viewport operations
    #[serde(rename = "viewport.change")]
    ViewportChange(ViewportParams),

    #[serde(rename = "diagnostics.change")]
    DiagnosticsChange(Vec<BufferDiagnostics>),

    #[serde(rename = "prompt.enter")]
    PromptEnter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct BufferDiagnostics {
    pub path: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct Diagnostic {
    pub range: Range,
    pub message: String,
    pub severity: Option<DiagnosticSeverity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub enum DiagnosticSeverity {
    Warning,
    Hint,
    Information,
    Error,
}

// Output Messages (Ki -> Host)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
#[serde(tag = "tag", content = "params")]
pub enum OutputMessage {
    // System operations
    #[serde(rename = "ping")]
    Ping(String),
    #[serde(rename = "error")]
    Error(String),

    // Buffer operations
    #[serde(rename = "buffer.open")]
    /// TODO: handle this on VS Code side
    /// See https://code.visualstudio.com/api/extension-guides/virtual-documents
    BufferOpen(BufferParams),
    #[serde(rename = "buffer.save")]
    BufferSave(BufferParams),
    #[serde(rename = "buffer.diff")]
    BufferDiff(BufferDiffParams),

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

    // Others
    #[serde(rename = "prompt.opened")]
    PromptOpened(PromptOpenedParams),
    #[serde(rename = "editor.jump")]
    JumpsChanged(JumpsParams),
    #[serde(rename = "editor.mark")]
    MarksChanged(MarksParams),

    // LSP actions
    #[serde(rename = "lsp.definition")]
    RequestLspDefinition,
    #[serde(rename = "lsp.hover")]
    RequestLspHover,
    #[serde(rename = "lsp.references")]
    RequestLspReferences,
    #[serde(rename = "lsp.declaration")]
    RequestLspDeclaration,
    #[serde(rename = "lsp.typeDefinition")]
    RequestLspTypeDefinition,
    #[serde(rename = "lsp.implementation")]
    RequestLspImplementation,
    #[serde(rename = "editor.keyboardLayout")]
    KeyboardLayoutChanged(String),
    #[serde(rename = "lsp.rename")]
    RequestLspRename,
    #[serde(rename = "lsp.codeAction")]
    RequestLspCodeAction,
    #[serde(rename = "lsp.documentSymbols")]
    RequestLspDocumentSymbols,
    #[serde(rename = "editor.syncBufferRequest")]
    SyncBufferRequest { uri: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct PromptOpenedParams {
    pub title: String,
    pub items: Vec<PromptItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct PromptItem {
    pub label: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct JumpTarget {
    pub key: char,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct JumpsParams {
    pub uri: String,
    pub targets: Vec<JumpTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct MarksParams {
    pub uri: String,
    pub marks: Vec<Range>,
}

// Main message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct InputMessageWrapper {
    pub message: InputMessage,
    #[typeshare(typescript(type = "number"))]
    pub id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[typeshare]
pub struct OutputMessageWrapper {
    pub message: OutputMessage,
    #[typeshare(typescript(type = "number"))]
    pub id: u32,
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[typeshare]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[typeshare(typescript(type = "any | undefined"))]
    pub data: Option<serde_json::Value>,
}

pub trait MessageMethod {
    /// Dynamic method that gets the message name at runtime
    fn method_name(&self) -> Cow<'static, str>;
}

impl MessageMethod for InputMessage {
    fn method_name(&self) -> Cow<'static, str> {
        match self {
            Self::Ping(_) => Cow::Borrowed("ping"),
            Self::BufferOpen(_) => Cow::Borrowed("buffer.open"),
            Self::BufferChange(_) => Cow::Borrowed("buffer.change"),
            Self::BufferActive(_) => Cow::Borrowed("buffer.active"),
            Self::SelectionSet(_) => Cow::Borrowed("selection.set"),
            Self::KeyboardInput(_) => Cow::Borrowed("keyboard.input"),
            Self::ViewportChange(_) => Cow::Borrowed("viewport.change"),
            Self::DiagnosticsChange(_) => Cow::Borrowed("diagnostics.change"),
            Self::PromptEnter(_) => Cow::Borrowed("prompt.enter"),
            Self::SyncBufferResponse(_) => Cow::Borrowed("editor.syncBufferResponse"),
        }
    }
}

impl MessageMethod for OutputMessage {
    fn method_name(&self) -> Cow<'static, str> {
        match self {
            OutputMessage::Ping(_) => Cow::Borrowed("ping"),
            OutputMessage::Error(_) => Cow::Borrowed("error"),
            OutputMessage::BufferOpen(_) => Cow::Borrowed("buffer.open"),
            OutputMessage::BufferSave(_) => Cow::Borrowed("buffer.save"),
            OutputMessage::BufferDiff(_) => Cow::Borrowed("buffer.diff"),
            OutputMessage::SelectionUpdate(_) => Cow::Borrowed("selection.update"),
            OutputMessage::ModeChange(_) => Cow::Borrowed("mode.change"),
            OutputMessage::SelectionModeChange(_) => Cow::Borrowed("selection_mode.change"),
            OutputMessage::ViewportChange(_) => Cow::Borrowed("viewport.change"),
            OutputMessage::JumpsChanged(_) => Cow::Borrowed("editor.jump"),
            OutputMessage::PromptOpened(_) => Cow::Borrowed("prompt.opened"),
            OutputMessage::MarksChanged(_) => Cow::Borrowed("editor.mark"),
            OutputMessage::RequestLspDefinition => Cow::Borrowed("lsp.definition"),
            OutputMessage::RequestLspHover => Cow::Borrowed("lsp.hover"),
            OutputMessage::RequestLspReferences => Cow::Borrowed("lsp.references"),
            OutputMessage::RequestLspDeclaration => Cow::Borrowed("lsp.declaration"),
            OutputMessage::RequestLspTypeDefinition => Cow::Borrowed("lsp.typeDefinition"),
            OutputMessage::RequestLspImplementation => Cow::Borrowed("lsp.implementation"),
            OutputMessage::KeyboardLayoutChanged(_) => Cow::Borrowed("editor.keyboardLayout"),
            OutputMessage::RequestLspRename => Cow::Borrowed("lsp.rename"),
            OutputMessage::RequestLspCodeAction => Cow::Borrowed("lsp.codeAction"),
            OutputMessage::RequestLspDocumentSymbols => Cow::Borrowed("lsp.documentSymbols"),
            OutputMessage::SyncBufferRequest { .. } => Cow::Borrowed("editor.requestBufferContent"),
        }
    }
}
