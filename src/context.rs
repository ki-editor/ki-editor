use crate::{clipboard::Clipboard, themes::Theme};

pub struct Context {
    previous_searches: Vec<Search>,
    clipboard: Clipboard,
    clipboard_content: Option<String>,
    mode: Option<GlobalMode>,
    pub theme: Theme,
}

#[derive(Clone)]
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
        self.clipboard
            .get_content()
            .or_else(|| self.clipboard_content.clone())
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
}
