use std::{cell::RefCell, collections::HashMap, rc::Rc};

use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    clipboard::Clipboard,
    lsp::diagnostic::Diagnostic,
    quickfix_list::{QuickfixListItem, QuickfixLists},
    syntax_highlight::{GetHighlightConfig, Highlight},
    themes::Theme,
};

type TreeSitterGrammarId = String;
pub struct Context {
    previous_searches: Vec<Search>,
    clipboard: Clipboard,
    mode: Option<GlobalMode>,
    diagnostics: HashMap<CanonicalizedPath, Vec<Diagnostic>>,
    theme: Theme,
    quickfix_lists: Rc<RefCell<QuickfixLists>>,

    /// We have to cache the highlight configurations because they load slowly.
    tree_sitter_highlight_configs:
        HashMap<TreeSitterGrammarId, tree_sitter_highlight::HighlightConfiguration>,
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
    LiteralIgnoreCase,
}

impl SearchKind {
    pub fn display(&self) -> &'static str {
        match self {
            SearchKind::Literal => "Literal",
            SearchKind::Regex => "Regex",
            SearchKind::AstGrep => "AST Grep",
            SearchKind::LiteralIgnoreCase => "Blind Case",
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            previous_searches: Vec::new(),
            clipboard: Clipboard::new(),
            theme: Theme::default(),
            diagnostics: Default::default(),
            mode: None,
            quickfix_lists: Rc::new(RefCell::new(QuickfixLists::new())),
            tree_sitter_highlight_configs: HashMap::new(),
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quickfix_lists(&self) -> Rc<RefCell<QuickfixLists>> {
        self.quickfix_lists.clone()
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
        self.clipboard.get_content()
    }

    pub fn set_clipboard_content(&mut self, content: String) {
        self.clipboard.set_content(content.clone());
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

    pub fn clear_clipboard(&mut self) {
        self.clipboard.clear()
    }

    pub fn get_quickfix_items(&self, path: &CanonicalizedPath) -> Option<Vec<QuickfixListItem>> {
        self.quickfix_lists.borrow().current().map(|list| {
            list.items()
                .iter()
                .filter(|item| &item.location().path == path)
                .cloned()
                .collect()
        })
    }

    pub fn set_theme(self, theme: Theme) -> Self {
        Self { theme, ..self }
    }

    pub(crate) fn highlight(
        &mut self,
        language: shared::language::Language,
        source_code: &str,
    ) -> anyhow::Result<crate::syntax_highlight::HighlighedSpans> {
        let Some(grammar_id) = language.tree_sitter_grammar_id() else { return Ok(Default::default()) };
        let config = match self.tree_sitter_highlight_configs.get(&grammar_id) {
            Some(config) => config,
            None => {
                if let Some(highlight_config) = language.get_highlight_config()? {
                    self.tree_sitter_highlight_configs
                        .insert(grammar_id.clone(), highlight_config);
                    let get_error = || {
                        anyhow::anyhow!("Unreachable: should be able to obtain a value that is inserted to the HashMap")
                    };
                    self.tree_sitter_highlight_configs
                        .get(&grammar_id)
                        .ok_or_else(get_error)?
                } else {
                    return Ok(Default::default());
                }
            }
        };
        config.highlight(self.theme(), source_code)
    }
}
