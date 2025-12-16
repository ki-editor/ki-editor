use std::collections::{HashMap, HashSet};

use globset::Glob;

use indexmap::IndexSet;
use itertools::{Either, Itertools};
use shared::canonicalized_path::CanonicalizedPath;
use strum::IntoEnumIterator;

use crate::{
    app::{GlobalSearchConfigUpdate, LocalSearchConfigUpdate, Scope},
    char_index_range::CharIndexRange,
    clipboard::{Clipboard, CopiedTexts},
    components::{editor_keymap::KeyboardLayoutKind, prompt::PromptHistoryKey},
    list::grep::RegexConfig,
    persistence::{Persistence, WorkspaceSession},
    quickfix_list::{DiagnosticSeverityRange, Location, QuickfixListItem},
    selection::SelectionMode,
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
    prompt_histories: HashMap<PromptHistoryKey, IndexSet<String>>,
    last_non_contiguous_selection_mode: Option<Either<SelectionMode, GlobalMode>>,
    keyboard_layout_kind: KeyboardLayoutKind,
    location_history_backward: Vec<Location>,
    location_history_forward: Vec<Location>,

    // We use CanonicalizedPath instead of CanonicalizedPath because these paths
    // may be deleted during program execution, and CanonicalizedPath
    // requires the path to exist on the filesystem.
    marked_files: IndexSet<CanonicalizedPath>,

    marks: HashMap<CanonicalizedPath, Vec<CharIndexRange>>,

    /// This is true, for example, when Ki is running as a VS Code's extension
    is_running_as_embedded: bool,

    persistence: Option<Persistence>,
}

pub(crate) struct QuickfixListState {
    pub(crate) title: String,
    pub(crate) source: QuickfixListSource,
    pub(crate) current_item_index: usize,
}

pub(crate) enum QuickfixListSource {
    Diagnostic(DiagnosticSeverityRange),
    Mark,
    Custom(Vec<QuickfixListItem>),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum GlobalMode {
    QuickfixListItem,
}
impl GlobalMode {
    pub(crate) fn display(&self) -> String {
        match self {
            GlobalMode::QuickfixListItem => "QFIX".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Search {
    pub(crate) mode: LocalSearchConfigMode,
    pub(crate) search: String,
}

impl Context {
    #[cfg(test)]
    pub(crate) fn default() -> Self {
        Self::new(CanonicalizedPath::try_from(".").unwrap(), false, None)
    }

    pub(crate) fn persist_data(&mut self) {
        if let Some(persistence) = self.persistence.as_mut() {
            let current_working_directory = self.current_working_directory.to_path_buf();
            persistence.set_workspace_session(
                current_working_directory,
                WorkspaceSession {
                    marked_files: self
                        .marked_files
                        .iter()
                        .map(|path| path.to_path_buf().clone())
                        .collect_vec(),
                    marks: self
                        .marks
                        .iter()
                        .map(|(path, marks)| (path.to_path_buf().clone(), marks.clone()))
                        .collect(),
                    prompt_histories: self.prompt_histories.clone(),
                },
            );

            if let Err(error) = persistence.write() {
                log::error!("Failed to write persistence due to {error:?}")
            }
        }
    }

    pub(crate) fn extend_quickfix_list_items(&mut self, new_items: Vec<QuickfixListItem>) {
        if let Some(QuickfixListState {
            source: QuickfixListSource::Custom(items),
            ..
        }) = self.quickfix_list_state.as_mut()
        {
            items.extend(new_items)
        }
    }

    pub(crate) fn quickfix_list_items(&self) -> Vec<&QuickfixListItem> {
        if let Some(QuickfixListState {
            source: QuickfixListSource::Custom(items),
            ..
        }) = &self.quickfix_list_state
        {
            items.iter().collect()
        } else {
            Default::default()
        }
    }

    pub(crate) fn handle_applied_edits(
        &mut self,
        path: CanonicalizedPath,
        edits: Vec<crate::edit::Edit>,
    ) {
        if let Some(state) = self.quickfix_list_state.take() {
            self.quickfix_list_state = Some(match state.source {
                QuickfixListSource::Diagnostic(_) => state,
                QuickfixListSource::Mark => state,
                QuickfixListSource::Custom(quickfix_list_items) => {
                    let items = quickfix_list_items
                        .into_iter()
                        .filter_map(|item| {
                            if item.location().path == path {
                                edits
                                    .iter()
                                    .try_fold(item, |item, edit| item.apply_edit(edit))
                            } else {
                                Some(item)
                            }
                        })
                        .collect_vec();
                    QuickfixListState {
                        source: QuickfixListSource::Custom(items),
                        ..state
                    }
                }
            })
        }

        self.marks = std::mem::take(&mut self.marks)
            .into_iter()
            .map(|(p, marks)| {
                if p == path {
                    (
                        p,
                        marks
                            .into_iter()
                            .filter_map(|mark| {
                                edits
                                    .iter()
                                    .try_fold(mark, |mark, edit| mark.apply_edit(edit))
                            })
                            .collect(),
                    )
                } else {
                    (p, marks)
                }
            })
            .collect();
    }

    pub(crate) fn save_marks(&mut self, path: CanonicalizedPath, marks: Vec<CharIndexRange>) {
        let old_ranges = self
            .marks
            .get(&path)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<HashSet<_>>();

        let new_ranges = marks.into_iter().collect::<HashSet<_>>();

        // We take the symmetric difference between the old ranges and the new ranges
        // so that user can unmark existing mark
        self.marks.insert(
            path,
            new_ranges
                .symmetric_difference(&old_ranges)
                .cloned()
                .collect_vec(),
        );
    }

    pub(crate) fn get_marks(&self, path: Option<CanonicalizedPath>) -> Vec<CharIndexRange> {
        path.map(|path| self.marks.get(&path).cloned().unwrap_or_default().to_vec())
            .unwrap_or_default()
    }

    pub(crate) fn marks(&self) -> &HashMap<CanonicalizedPath, Vec<CharIndexRange>> {
        &self.marks
    }

    pub(crate) fn handle_file_renamed(
        &mut self,
        source: std::path::PathBuf,
        destination: CanonicalizedPath,
    ) {
        if let Some(path) = self
            .marked_files
            .iter()
            .find(|path| path.to_path_buf() == &source)
        {
            self.marked_files.shift_remove(&path.clone());
            self.marked_files.insert(destination);
        }
    }
}

impl Context {
    pub(crate) fn new(
        current_working_directory: CanonicalizedPath,
        is_running_as_embedded: bool,
        persistence: Option<Persistence>,
    ) -> Self {
        let marked_files = persistence
            .as_ref()
            .and_then(|persistence| {
                Some(
                    persistence
                        .get_marked_files(current_working_directory.to_path_buf())?
                        .into_iter()
                        .filter_map(|path| CanonicalizedPath::try_from(path).ok())
                        .collect(),
                )
            })
            .unwrap_or_default();

        let marks = persistence
            .as_ref()
            .and_then(|persistence| {
                Some(
                    persistence
                        .get_marks(current_working_directory.to_path_buf())?
                        .into_iter()
                        .filter_map(|(path, marks)| Some((path.try_into().ok()?, marks)))
                        .collect(),
                )
            })
            .unwrap_or_default();

        let prompt_histories = persistence
            .as_ref()
            .and_then(|persistence| {
                persistence.get_prompt_histories(current_working_directory.to_path_buf())
            })
            .unwrap_or_default();

        Self {
            clipboard: Clipboard::new(),
            theme: Theme::default(),
            mode: None,
            #[cfg(test)]
            highlight_configs: crate::syntax_highlight::HighlightConfigs::new(),
            current_working_directory,
            local_search_config: LocalSearchConfig::default(),
            global_search_config: GlobalSearchConfig::default(),
            quickfix_list_state: Default::default(),
            prompt_histories,
            last_non_contiguous_selection_mode: None,
            keyboard_layout_kind: crate::config::AppConfig::singleton().keyboard_layout_kind(),
            location_history_backward: Vec::new(),
            location_history_forward: Vec::new(),
            marked_files,
            is_running_as_embedded,
            persistence,
            marks,
        }
    }

    /// Checks if the contents in both the system clipboard and the app clipboard is the same
    pub(crate) fn clipboards_synced(&self) -> bool {
        let history_offset = 0;
        let Some(app_clipboard_content) = self.clipboard.get(history_offset) else {
            return false;
        };

        let Some(system_clipboard_content) = self.clipboard.get_from_system_clipboard().ok() else {
            return false;
        };

        app_clipboard_content == system_clipboard_content
    }

    pub(crate) fn add_clipboard_history(&mut self, item: CopiedTexts) {
        self.clipboard.add_clipboard_history(item)
    }

    /// Note: `history_offset` is ignored when `use_system_clipboard` is true.
    ///
    /// This method should never fail, if `use_system_clipboard` is true but
    /// the system clipboard is inaccessible, the app clipboard will be used.
    pub(crate) fn get_clipboard_content(&self, history_offset: isize) -> Option<CopiedTexts> {
        // Always use the system clipboard if the content of the system clipboard is no longer the same
        // with the content of the app clipboard
        let use_system_clipboard = !self.clipboards_synced();

        if use_system_clipboard {
            match self.clipboard.get_from_system_clipboard() {
                Ok(copied_texts) => return Some(copied_texts),
                Err(err) => {
                    log::error!("Context::get_clipboard_content: cannot access system clipboard due to {err:?}")
                }
            }
        }

        self.clipboard.get(history_offset)
    }

    pub(crate) fn set_clipboard_content(&mut self, contents: CopiedTexts) -> anyhow::Result<()> {
        self.clipboard.set(contents.clone())
    }
    pub(crate) fn mode(&self) -> Option<GlobalMode> {
        self.mode.clone()
    }
    pub(crate) fn set_mode(&mut self, mode: Option<GlobalMode>) {
        self.mode = mode.clone();
        if let Some(mode) = mode {
            self.last_non_contiguous_selection_mode = Some(Either::Right(mode))
        }
    }

    pub(crate) fn theme(&self) -> &Theme {
        &self.theme
    }

    pub(crate) fn set_theme(&mut self, theme: Theme) {
        self.theme = theme
    }

    #[cfg(test)]
    pub(crate) fn highlight(
        &mut self,
        language: shared::language::Language,
        source_code: &str,
    ) -> anyhow::Result<crate::syntax_highlight::HighlightedSpans> {
        use std::sync::atomic::AtomicUsize;

        self.highlight_configs
            .highlight(language, source_code, &AtomicUsize::new(0))
    }

    pub(crate) fn current_working_directory(&self) -> &CanonicalizedPath {
        &self.current_working_directory
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
            GlobalSearchConfigUpdate::Config(config) => self.global_search_config = config,
        };
        Ok(())
    }

    pub(crate) fn local_search_config(&self, scope: Scope) -> &LocalSearchConfig {
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

    pub(crate) fn set_quickfix_list_source(&mut self, title: String, source: QuickfixListSource) {
        self.quickfix_list_state = Some(QuickfixListState {
            title,
            source,
            current_item_index: 0,
        })
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

    pub(crate) fn get_prompt_history(&self, key: PromptHistoryKey) -> Vec<String> {
        self.prompt_histories
            .get(&key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect_vec()
    }

    pub(crate) fn set_last_non_contiguous_selection_mode(
        &mut self,
        selection_mode: Either<crate::selection::SelectionMode, GlobalMode>,
    ) {
        self.last_non_contiguous_selection_mode = Some(selection_mode)
    }

    pub(crate) fn last_non_contiguous_selection_mode(
        &self,
    ) -> Option<&Either<crate::selection::SelectionMode, GlobalMode>> {
        self.last_non_contiguous_selection_mode.as_ref()
    }

    pub(crate) fn keyboard_layout_kind(&self) -> &KeyboardLayoutKind {
        &self.keyboard_layout_kind
    }

    pub(crate) fn set_keyboard_layout_kind(&mut self, keyboard_layout_kind: KeyboardLayoutKind) {
        self.keyboard_layout_kind = keyboard_layout_kind
    }

    pub(crate) fn push_location_history(&mut self, location: Location, backward: bool) {
        if backward {
            self.location_history_backward.push(location);
            self.location_history_forward.clear();
        } else {
            self.location_history_forward.push(location);
        }
    }

    pub(crate) fn location_previous(&mut self) -> Option<Location> {
        self.location_history_backward.pop()
    }

    pub(crate) fn location_next(&mut self) -> Option<Location> {
        self.location_history_forward.pop()
    }

    pub(crate) fn get_marked_files(&self) -> Vec<&CanonicalizedPath> {
        self.marked_files.iter().collect()
    }

    /// Returns some path if we should focus another file.
    /// If the action is to unmark a file, and the file is not the only marked file left,
    /// then we return the nearest neighbor.
    pub(crate) fn toggle_path_mark(
        &mut self,
        path: CanonicalizedPath,
    ) -> Option<&CanonicalizedPath> {
        if let Some(index) = self.marked_files.get_index_of(&path) {
            self.unmark_path_impl(index, path)
        } else {
            let _ = self.mark_file(path);
            None
        }
    }

    pub(crate) fn mark_file(&mut self, path: CanonicalizedPath) -> (usize, bool) {
        self.marked_files.insert_sorted(path)
    }

    /// Returns true if the path to be removed is in the list
    pub(crate) fn unmark_path(&mut self, path: CanonicalizedPath) -> Option<&CanonicalizedPath> {
        if let Some(index) = self.marked_files.get_index_of(&path) {
            self.unmark_path_impl(index, path)
        } else {
            None
        }
    }

    fn unmark_path_impl(
        &mut self,
        index: usize,
        path: CanonicalizedPath,
    ) -> Option<&CanonicalizedPath> {
        let _ = self.marked_files.shift_remove(&path);
        self.marked_files
            .get_index(if index == self.marked_files.len() {
                index.saturating_sub(1)
            } else {
                index
            })
    }

    pub(crate) fn is_running_as_embedded(&self) -> bool {
        self.is_running_as_embedded
    }

    pub(crate) fn rename_path_mark(&mut self, from: &CanonicalizedPath, to: &CanonicalizedPath) {
        self.marked_files.shift_remove(from);
        self.marked_files.insert_sorted(to.clone());
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct GlobalSearchConfig {
    pub(crate) include_glob: Option<Glob>,
    pub(crate) exclude_glob: Option<Glob>,
    pub(crate) local_config: LocalSearchConfig,
}
impl GlobalSearchConfig {
    pub(crate) fn local_config(&self) -> &LocalSearchConfig {
        &self.local_config
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
    NamingConventionAgnostic,
}
impl LocalSearchConfigMode {
    pub(crate) fn display(&self) -> String {
        match self {
            LocalSearchConfigMode::Regex(regex) => regex.display(),

            LocalSearchConfigMode::AstGrep => "AST Grep".to_string(),
            LocalSearchConfigMode::NamingConventionAgnostic => {
                "Naming Convention Agnostic".to_string()
            }
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
            "{}{}{}",
            if self.escaped { "Literal" } else { "Regex" },
            if self.case_sensitive {
                " A=a".to_string()
            } else {
                String::new()
            },
            if self.match_whole_word {
                " [Ab]".to_string()
            } else {
                String::new()
            }
        )
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalSearchConfig {
    pub(crate) mode: LocalSearchConfigMode,
    search: Option<String>,
    replacement: Option<String>,
}

impl LocalSearchConfig {
    pub(crate) fn new(mode: LocalSearchConfigMode) -> Self {
        Self {
            mode,
            search: Default::default(),
            replacement: Default::default(),
        }
    }

    fn update(&mut self, update: LocalSearchConfigUpdate) {
        match update {
            #[cfg(test)]
            LocalSearchConfigUpdate::Mode(mode) => self.mode = mode,
            #[cfg(test)]
            LocalSearchConfigUpdate::Replacement(replacement) => {
                self.set_replacment(replacement);
            }
            #[cfg(test)]
            LocalSearchConfigUpdate::Search(search) => {
                self.set_search(search);
            }
            LocalSearchConfigUpdate::Config(config) => *self = config,
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

#[cfg(test)]
mod test_context {
    use std::collections::HashMap;

    use indexmap::IndexSet;
    use itertools::Itertools;
    use shared::canonicalized_path::CanonicalizedPath;

    use crate::{
        char_index_range::CharIndexRange, components::prompt::PromptHistoryKey, context::Context,
        persistence::Persistence, selection::CharIndex,
    };

    #[test]
    fn test_persistence() -> anyhow::Result<()> {
        let temp_data_file = tempfile::NamedTempFile::new()?.path().to_path_buf();
        std::fs::write(temp_data_file.clone(), "")?;

        let temp_cwd = tempfile::tempdir()?.path().to_path_buf();
        std::fs::create_dir_all(temp_cwd.clone())?;

        let random_file = tempfile::NamedTempFile::new()?.path().to_path_buf();
        std::fs::write(random_file.clone(), "foo")?;

        let marks = [(CharIndex(0)..CharIndex(2)).into()].to_vec();

        // Save data
        {
            let persistence = Persistence::load_or_default(temp_data_file.clone());
            let mut context = Context::new(temp_cwd.clone().try_into()?, false, Some(persistence));
            let mut index_set = IndexSet::new();
            index_set.insert("Github Light".to_string());
            context.toggle_path_mark(random_file.clone().try_into()?);
            context.save_marks(random_file.clone().try_into()?, marks.clone());
            context
                .prompt_histories
                .insert(PromptHistoryKey::Theme, index_set);
            context.persist_data()
        }

        // Load data
        {
            let persistence = Persistence::load_or_default(temp_data_file);
            let context = Context::new(temp_cwd.try_into()?, false, Some(persistence));
            let actual_marked_files = context
                .get_marked_files()
                .into_iter()
                .cloned()
                .collect_vec();

            let expected_marked_files = vec![CanonicalizedPath::try_from(random_file.clone())?];

            assert_eq!(actual_marked_files, expected_marked_files);

            let actual_marks = context.marks;

            let expected_marks: HashMap<CanonicalizedPath, Vec<CharIndexRange>> =
                [(random_file.try_into()?, marks)].into_iter().collect();

            assert_eq!(actual_marks, expected_marks);

            let actual_prompt_histories = context.prompt_histories;
            assert_eq!(
                actual_prompt_histories
                    .get(&PromptHistoryKey::Theme)
                    .unwrap()
                    .len(),
                1
            );
        }

        Ok(())
    }
}
