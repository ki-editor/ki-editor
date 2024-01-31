use std::{cell::RefCell, collections::HashMap, rc::Rc};

use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    app::SearchConfigUpdate,
    clipboard::Clipboard,
    list::grep::GrepConfig,
    lsp::diagnostic::Diagnostic,
    quickfix_list::{QuickfixListItem, QuickfixLists},
    syntax_highlight::HighlightConfigs,
    themes::Theme,
};

pub struct Context {
    previous_searches: Vec<Search>,
    clipboard: Clipboard,
    mode: Option<GlobalMode>,
    diagnostics: HashMap<CanonicalizedPath, Vec<Diagnostic>>,
    theme: Box<Theme>,
    quickfix_lists: Rc<RefCell<QuickfixLists>>,

    highlight_configs: HighlightConfigs,
    current_working_directory: Option<CanonicalizedPath>,
    search_config: SearchConfig,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GlobalMode {
    QuickfixListItem,
    SelectionHistoryFile,
    SelectionHistoryContiguous,
}
impl GlobalMode {
    pub fn display(&self) -> String {
        match self {
            GlobalMode::QuickfixListItem => "QUICKFIX LIST ITEM".to_string(),
            GlobalMode::SelectionHistoryFile => "SELECTION HISTORY (FILE)".to_string(),
            GlobalMode::SelectionHistoryContiguous => "SELECTION HISTORY (CONTIGUOUS)".to_string(),
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
    AstGrep,
    Literal,
    LiteralCaseSensitive,
    Regex,
    RegexCaseSensitive,
    Custom { mode: SearchConfigMode },
}

impl SearchKind {
    pub fn display(&self) -> String {
        match self {
            SearchKind::AstGrep => "AST Grep".to_string(),
            SearchKind::Literal => "Literal".to_string(),
            SearchKind::LiteralCaseSensitive => "Literal (Case-sensitive)".to_string(),
            SearchKind::Regex => "Regex".to_string(),
            SearchKind::RegexCaseSensitive => "Regex (Case-sensitive)".to_string(),
            SearchKind::Custom { mode } => mode.display(),
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            previous_searches: Vec::new(),
            clipboard: Clipboard::new(),
            theme: Box::<Theme>::default(),
            diagnostics: Default::default(),
            mode: None,
            quickfix_lists: Rc::new(RefCell::new(QuickfixLists::new())),
            highlight_configs: HighlightConfigs::new(),
            current_working_directory: None,
            search_config: SearchConfig::default(),
        }
    }
}

impl Context {
    pub fn new(current_working_directory: CanonicalizedPath) -> Self {
        Self {
            current_working_directory: Some(current_working_directory),
            ..Self::default()
        }
    }

    pub fn quickfix_lists(&self) -> Rc<RefCell<QuickfixLists>> {
        self.quickfix_lists.clone()
    }

    pub fn last_search(&self) -> Option<Search> {
        self.previous_searches.last().cloned()
    }

    pub fn set_search(&mut self, search: Search) {
        self.search_config.set_search(search.search.clone());
        self.previous_searches.push(search);
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
        self.mode = mode;
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
        Self {
            theme: Box::new(theme),
            ..self
        }
    }

    pub(crate) fn highlight(
        &mut self,
        language: shared::language::Language,
        source_code: &str,
    ) -> anyhow::Result<crate::syntax_highlight::HighlighedSpans> {
        self.highlight_configs
            .highlight(self.theme.clone(), language, source_code)
    }

    pub(crate) fn current_working_directory(&self) -> Option<&CanonicalizedPath> {
        self.current_working_directory.as_ref()
    }

    pub(crate) fn search_config(&self) -> &SearchConfig {
        &self.search_config
    }

    pub(crate) fn update_search_config(&mut self, update: SearchConfigUpdate) {
        self.search_config.update(update)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum SearchConfigMode {
    Regex(GrepConfig),
    AstGrep,
}
impl SearchConfigMode {
    fn display(&self) -> String {
        match self {
            SearchConfigMode::Regex(regex) => regex.display(),

            SearchConfigMode::AstGrep => "AST Grep".to_string(),
        }
    }
}

impl Default for SearchConfigMode {
    fn default() -> Self {
        Self::Regex(Default::default())
    }
}

impl GrepConfig {
    fn display(&self) -> String {
        format!(
            "{}{}",
            if self.escaped { "Literal" } else { "Regex" },
            parenthesize(
                [
                    self.case_sensitive.then_some("Case-sensitive".to_string()),
                    self.match_whole_word
                        .then_some("Match whole word".to_string()),
                ]
                .into_iter()
                .flatten()
                .collect_vec(),
            ),
        )
    }
}

fn parenthesize(values: Vec<String>) -> String {
    if values.is_empty() {
        "".to_string()
    } else {
        format!("({})", values.join(", "))
    }
}

#[derive(Default, Clone)]
pub struct SearchConfig {
    pub mode: SearchConfigMode,
    pub search: String,
}
impl SearchConfig {
    fn update(&mut self, update: SearchConfigUpdate) {
        match update {
            SearchConfigUpdate::SetMode(mode) => self.mode = mode,
        }
    }

    fn set_search(&mut self, search: String) {
        self.search = search
    }
}
