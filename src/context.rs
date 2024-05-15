use globset::Glob;

use indexmap::IndexSet;
use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    app::{GlobalSearchConfigUpdate, GlobalSearchFilterGlob, LocalSearchConfigUpdate, Scope},
    clipboard::Clipboard,
    list::grep::RegexConfig,
    quickfix_list::DiagnosticSeverityRange,
    themes::Theme,
};

pub struct Context {
    clipboard: Clipboard,
    mode: Option<GlobalMode>,
    theme: Box<Theme>,

    #[cfg(test)]
    highlight_configs: crate::syntax_highlight::HighlightConfigs,
    current_working_directory: Option<CanonicalizedPath>,
    local_search_config: LocalSearchConfig,
    global_search_config: GlobalSearchConfig,
    quickfix_list_state: Option<QuickfixListState>,
}

pub struct QuickfixListState {
    pub source: QuickfixListSource,
    pub current_item_index: usize,
}

pub enum QuickfixListSource {
    Diagnostic(DiagnosticSeverityRange),
    Bookmark,
    Custom,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GlobalMode {
    QuickfixListItem,
}
impl GlobalMode {
    pub(crate) fn display(&self) -> String {
        match self {
            GlobalMode::QuickfixListItem => "QUICKFIX LIST ITEM".to_string(),
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
            mode: None,
            #[cfg(test)]
            highlight_configs: crate::syntax_highlight::HighlightConfigs::new(),
            current_working_directory: None,
            local_search_config: LocalSearchConfig::default(),
            global_search_config: GlobalSearchConfig::default(),
            quickfix_list_state: Default::default(),
        }
    }
}

impl Context {
    pub(crate) fn new(current_working_directory: CanonicalizedPath) -> Self {
        Self {
            current_working_directory: Some(current_working_directory),
            ..Self::default()
        }
    }

    pub(crate) fn get_clipboard_content(&self) -> Option<String> {
        self.clipboard.get_content()
    }

    pub(crate) fn set_clipboard_content(&mut self, content: String) {
        self.clipboard.set_content(content.clone());
    }
    pub(crate) fn mode(&self) -> Option<GlobalMode> {
        self.mode.clone()
    }
    pub(crate) fn set_mode(&mut self, mode: Option<GlobalMode>) {
        self.mode = mode;
    }

    pub(crate) fn theme(&self) -> &Theme {
        &self.theme
    }

    #[cfg(test)]
    pub(crate) fn set_theme(self, theme: Theme) -> Self {
        Self {
            theme: Box::new(theme),
            ..self
        }
    }

    #[cfg(test)]
    pub(crate) fn highlight(
        &mut self,
        language: shared::language::Language,
        source_code: &str,
    ) -> anyhow::Result<crate::syntax_highlight::HighlighedSpans> {
        self.highlight_configs.highlight(language, source_code)
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
                                .set_include_glob(Glob::new(&glob)?)
                        }
                    }
                    GlobalSearchFilterGlob::Exclude => {
                        if !glob.is_empty() {
                            self.global_search_config
                                .set_exclude_glob(Glob::new(&glob)?)
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

    pub(crate) fn quickfix_list_state(&self) -> &Option<QuickfixListState> {
        &self.quickfix_list_state
    }

    pub(crate) fn set_quickfix_list_current_item_index(&mut self, current_item_index: usize) {
        if let Some(state) = self.quickfix_list_state.take() {
            self.quickfix_list_state = Some(QuickfixListState {
                current_item_index,
                ..state
            })
        }
    }

    pub(crate) fn set_quickfix_list_source(&mut self, source: QuickfixListSource) {
        self.quickfix_list_state = Some(QuickfixListState {
            source,
            current_item_index: 0,
        })
    }
}

#[derive(Default)]
pub struct GlobalSearchConfig {
    include_globs: IndexSet<Glob>,
    exclude_globs: IndexSet<Glob>,
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

    fn set_exclude_glob(&mut self, glob: Glob) {
        self.exclude_globs.shift_remove(&glob);
        self.exclude_globs.insert(glob);
    }

    fn set_include_glob(&mut self, glob: Glob) {
        self.include_globs.shift_remove(&glob);
        self.include_globs.insert(glob);
    }

    pub(crate) fn include_glob(&self) -> Option<Glob> {
        self.include_globs.last().cloned()
    }

    pub(crate) fn exclude_glob(&self) -> Option<Glob> {
        self.exclude_globs.last().cloned()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum LocalSearchConfigMode {
    Regex(RegexConfig),
    AstGrep,
    CaseAgnostic,
}
impl LocalSearchConfigMode {
    pub(crate) fn display(&self) -> String {
        match self {
            LocalSearchConfigMode::Regex(regex) => regex.display(),

            LocalSearchConfigMode::AstGrep => "AST Grep".to_string(),
            LocalSearchConfigMode::CaseAgnostic => "Case Agnostic".to_string(),
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
    #[cfg(test)]
    pub(crate) fn new(mode: LocalSearchConfigMode) -> Self {
        Self {
            mode,
            searches: Default::default(),
            replacements: Default::default(),
        }
    }

    fn update(&mut self, update: LocalSearchConfigUpdate) {
        match update {
            LocalSearchConfigUpdate::Mode(mode) => self.mode = mode,
            LocalSearchConfigUpdate::Replacement(replacement) => {
                self.set_replacment(replacement);
            }
            LocalSearchConfigUpdate::Search(search) => {
                self.set_search(search);
            }
        }
    }

    pub(crate) fn set_search(&mut self, search: String) -> &mut Self {
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

    pub(crate) fn require_tree_sitter(&self) -> bool {
        self.mode == LocalSearchConfigMode::AstGrep
    }
}
