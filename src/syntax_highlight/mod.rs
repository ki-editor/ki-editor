#[cfg(test)]
mod test;

use std::{
    collections::HashMap,
    ops::Range,
    sync::{atomic::AtomicUsize, mpsc::Sender},
    time::Duration,
};

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{
    app::AppMessage,
    components::component::ComponentId,
    grid::{IndexedHighlightGroup, StyleKey},
};
use shared::language::Language;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightedSpan {
    pub byte_range: Range<usize>,
    pub style_key: StyleKey,
}

pub trait GetHighlightConfig {
    fn get_highlight_config(&self) -> anyhow::Result<Option<HighlightConfiguration>>;
}

impl GetHighlightConfig for Language {
    fn get_highlight_config(&self) -> anyhow::Result<Option<HighlightConfiguration>> {
        let tree_sitter_language = if let Some(tree_sitter_language) = self.tree_sitter_language() {
            tree_sitter_language
        } else {
            return Ok(None);
        };

        let Some(highlights_query) = &self.highlight_query() else {
            return Ok(None);
        };
        let new_config = |injection_query: &str| {
            HighlightConfiguration::new(
                tree_sitter_language.clone(),
                "highlight".to_string(),
                highlights_query,
                injection_query,
                self.locals_query().unwrap_or_default(),
            )
        };
        // A malformed injection query must not disable highlighting of the host.
        let mut config = match self.injection_query() {
            Some(injection_query) => new_config(injection_query).or_else(|_| new_config(""))?,
            None => new_config("")?,
        };

        config.configure(crate::themes::highlight_names().as_slice());

        Ok(Some(config))
    }
}

pub trait Highlight {
    fn highlight(
        &self,
        source_code: &str,
        cancellation_flag: &AtomicUsize,
    ) -> anyhow::Result<HighlightedSpans>;
}

impl Highlight for HighlightConfiguration {
    fn highlight(
        &self,
        source_code: &str,
        cancellation_flag: &AtomicUsize,
    ) -> anyhow::Result<HighlightedSpans> {
        let mut highlighter = Highlighter::new();

        let highlights = highlighter.highlight(
            self,
            source_code.as_bytes(),
            Some(cancellation_flag),
            |_| None,
        )?;

        collect_highlighted_spans(highlights)
    }
}

fn collect_highlighted_spans(
    highlights: impl Iterator<Item = Result<HighlightEvent, tree_sitter_highlight::Error>>,
) -> anyhow::Result<HighlightedSpans> {
    let (_, highlighted_spans) = highlights.into_iter().try_fold(
        (Vec::new(), Vec::new()),
        |(mut highlight_events, mut highlighted_spans), event| -> anyhow::Result<_> {
            match event? {
                HighlightEvent::HighlightStart(s) => {
                    highlight_events.push(s);
                }
                HighlightEvent::HighlightEnd => {
                    highlight_events.pop();
                }
                HighlightEvent::Source { start, end } => {
                    if let Some(highlight) = highlight_events.last() {
                        let style_key = StyleKey::Syntax(IndexedHighlightGroup::new(highlight.0));
                        highlighted_spans.push(HighlightedSpan {
                            byte_range: start..end,
                            style_key,
                        });
                    }
                }
            }
            Ok((highlight_events, highlighted_spans))
        },
    )?;

    debug_assert!(highlighted_spans
        .iter()
        .is_sorted_by_key(|span| (span.byte_range.start, span.byte_range.end)));

    Ok(HighlightedSpans(highlighted_spans))
}

#[derive(Clone, Default, Debug)]
pub struct HighlightedSpans(pub Vec<HighlightedSpan>);
impl HighlightedSpans {
    /// This method only updates the highlight spans within the affected range.
    /// The affected range starts from the smallest point of edit to the last visible range.
    ///
    /// We don't update highlight spans that are out of the visible bounds because:
    /// 1. That is expensive due to the huge number of highlight spans
    /// 2. The highlight spans will be recomputed quickly, so there's no point
    ///    in updating the out-of-bound ones.
    pub fn apply_edit_mut(&mut self, affected_range: &Range<usize>, change: isize) {
        if self.0.is_empty() {
            return;
        }
        let length = self.0.len();
        let start_index = self
            .0
            .partition_point(|span| span.byte_range.end <= affected_range.start);

        let end_index = self
            .0
            .partition_point(|span| span.byte_range.start < affected_range.end);

        if start_index >= length {
            return;
        }
        self.0[start_index..end_index.max(start_index)]
            .iter_mut()
            .for_each(|span| {
                span.byte_range.start = (span.byte_range.start as isize + change) as usize;
                span.byte_range.end = (span.byte_range.end as isize + change) as usize;
            });
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct SyntaxHighlightRequestBatchId(u8);

impl SyntaxHighlightRequestBatchId {
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Debug)]
pub struct SyntaxHighlightRequest {
    pub component_id: ComponentId,
    pub batch_id: SyntaxHighlightRequestBatchId,
    pub language: Language,
    pub source_code: String,
}

pub fn start_thread(
    callback: crossbeam_channel::Sender<AppMessage>,
) -> Sender<SyntaxHighlightRequest> {
    let (sender, receiver) = std::sync::mpsc::channel::<SyntaxHighlightRequest>();
    use debounce::EventDebouncer;
    struct Event(SyntaxHighlightRequest);
    impl PartialEq for Event {
        fn eq(&self, other: &Self) -> bool {
            self.0.component_id == other.0.component_id
        }
    }

    use std::cell::RefCell;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    std::thread::spawn(move || {
        let mut highlight_configs = HighlightConfigs::new();
        // Use Arc<AtomicUsize> which can be cloned

        let last_cancellation_flag = RefCell::new(None::<Arc<AtomicUsize>>);

        let debounce = EventDebouncer::new(Duration::from_millis(150), move |Event(request)| {
            // Cancel the previous operation if it exists
            // The cancellation is done by unzeroing the atomic usize
            if let Some(flag) = last_cancellation_flag.borrow_mut().take() {
                flag.fetch_add(1, Ordering::Relaxed);
            }

            // Create a new cancellation flag for this operation
            let new_cancellation_flag = Arc::new(AtomicUsize::new(0));

            // Store a clone of the new flag for potential cancellation in the future
            *last_cancellation_flag.borrow_mut() = Some(new_cancellation_flag.clone());

            match highlight_configs.highlight(
                request.language,
                &request.source_code,
                &new_cancellation_flag,
            ) {
                Ok(highlighted_spans) => {
                    let _ = callback.send(AppMessage::SyntaxHighlightResponse {
                        component_id: request.component_id,
                        batch_id: request.batch_id,
                        highlighted_spans,
                    });
                }
                Err(error) => {
                    log::info!("syntax_highlight_error = {error:#?}");
                }
            }
        });

        while let Ok(request) = receiver.recv() {
            debounce.put(Event(request));
        }
    });

    sender
}
type TreeSitterGrammarId = String;
/// We have to cache the highlight configurations because they load slowly.
#[derive(Default)]
pub struct HighlightConfigs(
    HashMap<TreeSitterGrammarId, tree_sitter_highlight::HighlightConfiguration>,
);

impl HighlightConfigs {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_highlight_config(
        &mut self,
        language: Language,
    ) -> anyhow::Result<Option<TreeSitterGrammarId>> {
        let Some(grammar_id) = language.tree_sitter_grammar_id() else {
            return Ok(None);
        };
        if self.0.contains_key(&grammar_id) {
            return Ok(Some(grammar_id));
        }

        let Some(highlight_config) = language.get_highlight_config()? else {
            return Ok(None);
        };
        self.0.insert(grammar_id.clone(), highlight_config);
        Ok(Some(grammar_id))
    }

    pub fn highlight(
        &mut self,
        language: Language,
        source_code: &str,
        cancellation_flag: &AtomicUsize,
    ) -> Result<HighlightedSpans, anyhow::Error> {
        let Some(grammar_id) = self.ensure_highlight_config(language.clone())? else {
            return Ok(HighlightedSpans::default());
        };

        language
            .injected_language_ids()
            .filter_map(language_from_injection_name)
            // A broken injected language must not disable highlighting of the host.
            .for_each(|language| {
                let _ = self.ensure_highlight_config(language);
            });

        let configs = &self.0;
        let config = configs.get(&grammar_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Unreachable: should be able to obtain a cached highlight configuration"
            )
        })?;

        let mut highlighter = Highlighter::new();
        let highlights = highlighter.highlight(
            config,
            source_code.as_bytes(),
            Some(cancellation_flag),
            |injection_name| {
                language_from_injection_name(injection_name)
                    .and_then(|language| language.tree_sitter_grammar_id())
                    .and_then(|grammar_id| configs.get(&grammar_id))
            },
        )?;

        collect_highlighted_spans(highlights)
    }
}

pub(crate) fn language_from_injection_name(name: &str) -> Option<Language> {
    let language_key = match name {
        "js" | "javascript" => "javascript",
        "jsx" => "javascriptreact",
        "ts" | "typescript" => "typescript",
        "tsx" => "typescriptreact",
        "css" => "css",
        "scss" => "scss",
        _ => name,
    };
    crate::config::AppConfig::singleton()
        .languages()
        .get(language_key)
        .cloned()
}
