use std::collections::HashMap;

use globset::Glob;

use indexmap::IndexSet;
use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{
    app::{GlobalSearchConfigUpdate, GlobalSearchFilterGlob, LocalSearchConfigUpdate, Scope},
    clipboard::Clipboard,
    components::{keymap_legend::KeymapLegendSection, prompt::PromptHistoryKey},
    list::grep::RegexConfig,
    quickfix_list::DiagnosticSeverityRange,
    themes::Theme,
};

pub(crate) struct Context {
    clipboard: Clipboard,
    mode: Option<GlobalMode>,
    theme: Theme,

    #[cfg(test)]
    highlight_configs: crate::syntax_highlight::HighlightConfigs,
    current_working_directory: CanonicalizedPath,
    local_search_config: LocalSearchConfig,
    global_search_config: GlobalSearchConfig,
    quickfix_list_state: Option<QuickfixListState>,
    contextual_keymaps: Vec<KeymapLegendSection>,
    prompt_histories: HashMap<PromptHistoryKey, IndexSet<String>>,
}

pub(crate) struct QuickfixListState {
    pub(crate) source: QuickfixListSource,
    pub(crate) current_item_index: usize,
}

pub(crate) enum QuickfixListSource {
    Diagnostic(DiagnosticSeverityRange),
    Bookmark,
    Custom,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum GlobalMode {
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
pub(crate) struct Search {
    pub(crate) mode: LocalSearchConfigMode,
    pub(crate) search: String,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            clipboard: Clipboard::new(),
            theme: Theme::default(),
            mode: None,
            #[cfg(test)]
            highlight_configs: crate::syntax_highlight::HighlightConfigs::new(),
            current_working_directory: CanonicalizedPath::try_from(".").unwrap(),
            local_search_config: LocalSearchConfig::default(),
            global_search_config: GlobalSearchConfig::default(),
            quickfix_list_state: Default::default(),
            contextual_keymaps: Default::default(),
            prompt_histories: Default::default(),
        }
    }
}

impl Context {
    pub(crate) fn new(current_working_directory: CanonicalizedPath) -> Self {
        Self {
            current_working_directory,
            ..Self::default()
        }
    }

    pub(crate) fn get_clipboard_content(&self, history_offset: isize) -> Option<Vec<String>> {
        self.clipboard.get(history_offset)
    }

    pub(crate) fn get_from_system_clipboard(&self) -> anyhow::Result<String> {
        self.clipboard.get_from_system_clipboard()
    }

    pub(crate) fn set_clipboard_content(
        &mut self,
        contents: Vec<String>,
        use_system_clipboard: bool,
    ) -> anyhow::Result<()> {
        self.clipboard.set(contents.clone(), use_system_clipboard)
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

    pub(crate) fn set_theme(self, theme: Theme) -> Self {
        Self { theme, ..self }
    }

    #[cfg(test)]
    pub(crate) fn highlight(
        &mut self,
        language: shared::language::Language,
        source_code: &str,
    ) -> anyhow::Result<crate::syntax_highlight::HighlighedSpans> {
        self.highlight_configs.highlight(language, source_code)
    }

    pub(crate) fn current_working_directory(&self) -> &CanonicalizedPath {
        &self.current_working_directory
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

    pub(crate) fn contextual_keymaps(&self) -> Vec<KeymapLegendSection> {
        self.contextual_keymaps.clone()
    }

    pub(crate) fn set_contextual_keymaps(&mut self, contextual_keymaps: Vec<KeymapLegendSection>) {
        self.contextual_keymaps = contextual_keymaps
    }

    pub(crate) fn push_history_prompt(&mut self, key: PromptHistoryKey, line: String) {
        if let Some(map) = self.prompt_histories.get_mut(&key) {
            map.shift_remove(&line);
            let inserted = map.insert(line);
            debug_assert!(inserted);
        } else {
            self.prompt_histories.insert(key, {
                let mut set = IndexSet::new();
                set.insert(line);
                set
            });
        }
    }

    pub(crate) fn get_prompt_history(
        &mut self,
        key: PromptHistoryKey,
        current_entry: Option<String>,
    ) -> Vec<String> {
        if let Some(line) = current_entry {
            self.push_history_prompt(key, line)
        }
        self.prompt_histories
            .get(&key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect_vec()
    }
}

#[derive(Default)]
pub(crate) struct GlobalSearchConfig {
    include_glob: Option<Glob>,
    exclude_glob: Option<Glob>,
    local_config: LocalSearchConfig,
}
impl GlobalSearchConfig {
    pub(crate) fn local_config(&self) -> &LocalSearchConfig {
        &self.local_config
    }

    fn set_exclude_glob(&mut self, glob: Glob) {
        let _ = self.exclude_glob.insert(glob);
    }

    fn set_include_glob(&mut self, glob: Glob) {
        let _ = self.include_glob.insert(glob);
    }

    pub(crate) fn include_glob(&self) -> Option<Glob> {
        self.include_glob.clone()
    }

    pub(crate) fn exclude_glob(&self) -> Option<Glob> {
        self.exclude_glob.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub(crate) enum LocalSearchConfigMode {
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
pub(crate) struct LocalSearchConfig {
    pub(crate) mode: LocalSearchConfigMode,
    search: Option<String>,
    replacement: Option<String>,
}

impl LocalSearchConfig {
    #[cfg(test)]
    pub(crate) fn new(mode: LocalSearchConfigMode) -> Self {
        Self {
            mode,
            search: Default::default(),
            replacement: Default::default(),
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
        let _ = self.search.insert(search);
        self
    }

    pub(crate) fn search(&self) -> String {
        self.search.clone().unwrap_or_default()
    }

    pub(crate) fn set_replacment(&mut self, replacement: String) -> &mut Self {
        let _ = self.replacement.insert(replacement);
        self
    }

    pub(crate) fn last_search(&self) -> Option<Search> {
        self.search.clone().map(|search| Search {
            search,
            mode: self.mode,
        })
    }

    pub(crate) fn replacement(&self) -> String {
        self.replacement.clone().unwrap_or_default()
    }

    pub(crate) fn require_tree_sitter(&self) -> bool {
        self.mode == LocalSearchConfigMode::AstGrep
    }
}
