use std::{cell::RefCell, collections::HashMap, rc::Rc};

use indexmap::IndexSet;
use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    app::{GlobalSearchConfigUpdate, GlobalSearchFilterGlob, LocalSearchConfigUpdate, Scope},
    clipboard::Clipboard,
    list::grep::RegexConfig,
    lsp::diagnostic::Diagnostic,
    quickfix_list::{QuickfixListItem, QuickfixLists},
    syntax_highlight::HighlightConfigs,
    themes::Theme,
};

pub struct Context {
    clipboard: Clipboard,
    mode: Option<GlobalMode>,
    diagnostics: HashMap<CanonicalizedPath, Vec<Diagnostic>>,
    theme: Box<Theme>,
    quickfix_lists: Rc<RefCell<QuickfixLists>>,

    highlight_configs: HighlightConfigs,
    current_working_directory: Option<CanonicalizedPath>,
    local_search_config: LocalSearchConfig,
    global_search_config: GlobalSearchConfig,
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
    pub mode: LocalSearchConfigMode,
    pub search: String,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            clipboard: Clipboard::new(),
            theme: Box::<Theme>::default(),
            diagnostics: Default::default(),
            mode: None,
            quickfix_lists: Rc::new(RefCell::new(QuickfixLists::new())),
            highlight_configs: HighlightConfigs::new(),
            current_working_directory: None,
            local_search_config: LocalSearchConfig::default(),
            global_search_config: GlobalSearchConfig::default(),
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

    pub(crate) fn local_search_config(&self) -> &LocalSearchConfig {
        &self.local_search_config
    }

    pub(crate) fn global_search_config(&self) -> &GlobalSearchConfig {
        &self.global_search_config
    }

    pub(crate) fn update_local_search_config(
        &mut self,
        update: LocalSearchConfigUpdate,
        scope: Scope,
    ) {
        match scope {
            Scope::Local => &mut self.local_search_config,
            Scope::Global => &mut self.global_search_config.local_config,
        }
        .update(update)
    }

    pub(crate) fn update_global_search_config(
        &mut self,
        update: GlobalSearchConfigUpdate,
    ) -> anyhow::Result<()> {
        match update {
            GlobalSearchConfigUpdate::SetGlob(which, glob) => {
                match which {
                    GlobalSearchFilterGlob::Include => {
                        if !glob.is_empty() {
                            self.global_search_config
                                .set_include_glob(glob::Pattern::new(&glob)?)
                        }
                    }
                    GlobalSearchFilterGlob::Exclude => {
                        if !glob.is_empty() {
                            self.global_search_config
                                .set_exclude_glob(glob::Pattern::new(&glob)?)
                        }
                    }
                };
            }
        };
        Ok(())
    }

    pub(crate) fn get_local_search_config(&self, scope: Scope) -> &LocalSearchConfig {
        match scope {
            Scope::Local => &self.local_search_config,
            Scope::Global => &self.global_search_config.local_config,
        }
    }
}

#[derive(Default)]
pub struct GlobalSearchConfig {
    include_globs: IndexSet<glob::Pattern>,
    exclude_globs: IndexSet<glob::Pattern>,
    local_config: LocalSearchConfig,
}
impl GlobalSearchConfig {
    pub(crate) fn local_config(&self) -> &LocalSearchConfig {
        &self.local_config
    }

    pub(crate) fn include_globs(&self) -> Vec<String> {
        self.include_globs
            .iter()
            .map(|glob| glob.to_string())
            .collect()
    }

    pub(crate) fn exclude_globs(&self) -> Vec<String> {
        self.exclude_globs
            .iter()
            .map(|glob| glob.to_string())
            .collect()
    }

    fn set_exclude_glob(&mut self, glob: glob::Pattern) {
        self.exclude_globs.shift_remove(&glob);
        self.exclude_globs.insert(glob);
    }

    fn set_include_glob(&mut self, glob: glob::Pattern) {
        self.include_globs.shift_remove(&glob);
        self.include_globs.insert(glob);
    }

    pub(crate) fn include_glob(&self) -> Option<glob::Pattern> {
        self.include_globs.last().cloned()
    }

    pub(crate) fn exclude_glob(&self) -> Option<glob::Pattern> {
        self.exclude_globs.last().cloned()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum LocalSearchConfigMode {
    Regex(RegexConfig),
    AstGrep,
}
impl LocalSearchConfigMode {
    pub fn display(&self) -> String {
        match self {
            LocalSearchConfigMode::Regex(regex) => regex.display(),

            LocalSearchConfigMode::AstGrep => "AST Grep".to_string(),
        }
    }
}

impl Default for LocalSearchConfigMode {
    fn default() -> Self {
        Self::Regex(Default::default())
    }
}

impl RegexConfig {
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

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct LocalSearchConfig {
    pub mode: LocalSearchConfigMode,
    searches: IndexSet<String>,
    replacements: IndexSet<String>,
}

impl LocalSearchConfig {
    pub fn new(mode: LocalSearchConfigMode) -> Self {
        Self {
            mode,
            searches: Default::default(),
            replacements: Default::default(),
        }
    }

    fn update(&mut self, update: LocalSearchConfigUpdate) {
        match update {
            LocalSearchConfigUpdate::SetMode(mode) => self.mode = mode,
            LocalSearchConfigUpdate::SetReplacement(replacement) => {
                self.set_replacment(replacement);
            }
            LocalSearchConfigUpdate::SetSearch(search) => {
                self.set_search(search);
            }
        }
    }

    pub fn set_search(&mut self, search: String) -> &mut Self {
        self.searches.shift_remove(&search);
        self.searches.insert(search);
        self
    }

    pub(crate) fn search(&self) -> String {
        self.searches.last().cloned().unwrap_or_default()
    }

    pub(crate) fn set_replacment(&mut self, replacement: String) -> &mut Self {
        self.replacements.shift_remove(&replacement);
        self.replacements.insert(replacement);
        self
    }

    pub(crate) fn last_search(&self) -> Option<Search> {
        self.searches.last().cloned().map(|search| Search {
            search,
            mode: self.mode,
        })
    }

    pub(crate) fn searches(&self) -> Vec<String> {
        self.searches.clone().into_iter().collect()
    }

    pub(crate) fn replacement(&self) -> String {
        self.replacements.last().cloned().unwrap_or_default()
    }

    pub(crate) fn replacements(&self) -> Vec<String> {
        self.replacements.clone().into_iter().collect()
    }
}
