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
pub(crate) struct HighlightedSpan {
    pub(crate) byte_range: Range<usize>,
    pub(crate) style_key: StyleKey,
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
        let mut config = HighlightConfiguration::new(
            tree_sitter_language,
            "highlight".to_string(),
            highlights_query,
            self.injection_query().unwrap_or_default(),
            self.locals_query().unwrap_or_default(),
        )?;

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

        let mut highlight = None;

        let mut highlighted_spans = vec![];

        for event in highlights {
            match event? {
                HighlightEvent::HighlightStart(s) => {
                    highlight = Some(s);
                }
                HighlightEvent::HighlightEnd => {
                    highlight = None;
                }
                HighlightEvent::Source { start, end } => {
                    if let Some(highlight) = highlight {
                        let style_key = StyleKey::Syntax(IndexedHighlightGroup::new(highlight.0));
                        highlighted_spans.push(HighlightedSpan {
                            byte_range: start..end,
                            style_key,
                        });
                    }
                }
            }
        }

        debug_assert!(highlighted_spans
            .iter()
            .is_sorted_by_key(|span| (span.byte_range.start, span.byte_range.end)));

        Ok(HighlightedSpans(highlighted_spans))
    }
}

#[derive(Clone, Default, Debug)]
pub(crate) struct HighlightedSpans(pub Vec<HighlightedSpan>);
impl HighlightedSpans {
    /// This method only updates the highlight spans within the affected range.
    /// The affected range starts from the smallest point of edit to the last visible range.
    ///
    /// We don't update highlight spans that are out of the visible bounds because:
    /// 1. That is expensive due to the huge number of highlight spans
    /// 2. The highlight spans will be recomputed quickly, so there's no point
    ///    in updating the out-of-bound ones.
    pub(crate) fn apply_edit_mut(&mut self, affected_range: &Range<usize>, change: isize) {
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
pub(crate) struct SyntaxHighlightRequestBatchId(u8);

impl SyntaxHighlightRequestBatchId {
    pub(crate) fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1)
    }
}

#[derive(Debug)]
pub(crate) struct SyntaxHighlightRequest {
    pub(crate) component_id: ComponentId,
    pub(crate) batch_id: SyntaxHighlightRequestBatchId,
    pub(crate) language: Language,
    pub(crate) source_code: String,
}

pub(crate) fn start_thread(callback: Sender<AppMessage>) -> Sender<SyntaxHighlightRequest> {
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
                    log::info!("syntax_highlight_error = {error:#?}")
                }
            }
        });

        while let Ok(request) = receiver.recv() {
            debounce.put(Event(request))
        }
    });

    sender
}
type TreeSitterGrammarId = String;
/// We have to cache the highlight configurations because they load slowly.
pub(crate) struct HighlightConfigs(
    HashMap<TreeSitterGrammarId, tree_sitter_highlight::HighlightConfiguration>,
);

impl HighlightConfigs {
    pub(crate) fn new() -> HighlightConfigs {
        HighlightConfigs(Default::default())
    }

    pub(crate) fn highlight(
        &mut self,
        language: Language,
        source_code: &str,
        cancellation_flag: &AtomicUsize,
    ) -> Result<HighlightedSpans, anyhow::Error> {
        let Some(grammar_id) = language.tree_sitter_grammar_id() else {
            return Ok(Default::default());
        };
        let config = match self.0.get(&grammar_id) {
            Some(config) => config,
            None => {
                if let Some(highlight_config) = language.get_highlight_config()? {
                    self.0.insert(grammar_id.clone(), highlight_config);
                    let get_error = || {
                        anyhow::anyhow!("Unreachable: should be able to obtain a value that is inserted to the HashMap")
                    };
                    self.0.get(&grammar_id).ok_or_else(get_error)?
                } else {
                    return Ok(Default::default());
                }
            }
        };
        config.highlight(source_code, cancellation_flag)
    }
}
