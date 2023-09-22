use std::collections::HashMap;

use shared::canonicalized_path::CanonicalizedPath;

use crate::{clipboard::Clipboard, lsp::diagnostic::Diagnostic, themes::Theme};

#[derive(Clone)]
pub struct Context {
    previous_searches: Vec<Search>,
    clipboard: Clipboard,
    clipboard_content: Option<String>,
    mode: Option<GlobalMode>,
    diagnostics: HashMap<CanonicalizedPath, Vec<Diagnostic>>,
    theme: Theme,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GlobalMode {
    QuickfixListItem,
    BufferNavigationHistory,
}
impl GlobalMode {
    pub fn display(&self) -> String {
        match self {
            GlobalMode::QuickfixListItem => "QUICKFIX LIST ITEM".to_string(),
            GlobalMode::BufferNavigationHistory => "BUFFER NAVIGATION HISTORY".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Search {
    pub kind: SearchKind,
    pub search: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum SearchKind {
    Literal,
    Regex,
    AstGrep,
    IgnoreCase,
}

impl SearchKind {
    pub fn display(&self) -> &'static str {
        match self {
            SearchKind::Literal => "Literal",
            SearchKind::Regex => "Regex",
            SearchKind::AstGrep => "AST Grep",
            SearchKind::IgnoreCase => "Ignore Case",
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            previous_searches: Vec::new(),
            clipboard: Clipboard::new(),
            clipboard_content: None,
            theme: Theme::default(),
            diagnostics: Default::default(),
            mode: None,
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            previous_searches: Vec::new(),
            clipboard: Clipboard::new(),
            clipboard_content: None,
            theme: Theme::default(),
            mode: None,
            diagnostics: Default::default(),
        }
    }
    pub fn last_search(&self) -> Option<Search> {
        self.previous_searches.last().cloned()
    }

    pub fn set_search(&mut self, search: Search) {
        self.previous_searches.push(search)
    }

    pub fn previous_searches(&self) -> Vec<Search> {
        self.previous_searches.clone()
    }

    pub fn get_clipboard_content(&self) -> Option<String> {
        let result = self
            .clipboard
            .get_content()
            .or_else(|| self.clipboard_content.clone());
        log::info!("get_clipboard_content = {result:#?}");
        result
    }

    pub fn set_clipboard_content(&mut self, content: String) {
        self.clipboard.set_content(content.clone());
        self.clipboard_content = Some(content);
    }
    pub fn mode(&self) -> Option<GlobalMode> {
        self.mode.clone()
    }
    pub fn set_mode(&mut self, mode: Option<GlobalMode>) {
        self.mode = mode
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn diagnostics(&self) -> Vec<(&CanonicalizedPath, &Diagnostic)> {
        self.diagnostics
            .iter()
            .flat_map(|(path, diagnostics)| {
                diagnostics.iter().map(move |diagnostic| (path, diagnostic))
            })
            .collect::<Vec<_>>()
    }

    pub fn update_diagnostics(&mut self, path: CanonicalizedPath, diagnostics: Vec<Diagnostic>) {
        self.diagnostics.insert(path, diagnostics);
    }

    pub fn get_diagnostics(&self, path: Option<CanonicalizedPath>) -> Vec<&Diagnostic> {
        path.map(|path| {
            self.diagnostics
                .get(&path)
                .map(|diagnostics| diagnostics.iter().collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .unwrap_or_default()
    }
}
