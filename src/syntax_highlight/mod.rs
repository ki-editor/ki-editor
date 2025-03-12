use std::{collections::HashMap, ops::Range, sync::mpsc::Sender, time::Duration};

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{
    app::AppMessage,
    char_index_range::apply_edit,
    components::component::ComponentId,
    grid::{IndexedHighlightGroup, StyleKey},
    themes::highlight_names,
};
use shared::language::Language;

#[derive(Clone, Debug)]
pub(crate) struct HighlightedSpan {
    pub(crate) byte_range: Range<usize>,
    pub(crate) style_key: StyleKey,
}
impl HighlightedSpan {
    fn apply_edit(self, edited_range: &Range<usize>, change: isize) -> Option<HighlightedSpan> {
        Some(HighlightedSpan {
            byte_range: apply_edit(self.byte_range, edited_range, change)?,
            ..self
        })
    }
    /// Return `true` if this `HighlightedSpan` should be retained after applying the edit
    fn apply_edit_mut(&mut self, edited_range: &Range<usize>, change: isize) -> bool {
        let byte_range = std::mem::take(&mut self.byte_range);
        if let Some(byte_range) = apply_edit(byte_range, edited_range, change) {
            self.byte_range = byte_range;
            true
        } else {
            false
        }
    }
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

        let highlights_query = &self.highlight_query().unwrap_or_default();
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
    fn highlight(&self, source_code: &str) -> anyhow::Result<HighlightedSpans>;
}

impl Highlight for HighlightConfiguration {
    fn highlight(&self, source_code: &str) -> anyhow::Result<HighlightedSpans> {
        let mut highlighter = Highlighter::new();

        let highlights = highlighter.highlight(self, source_code.as_bytes(), None, |_| None)?;

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
        Ok(HighlightedSpans(highlighted_spans))
    }
}

#[derive(Clone, Default, Debug)]
pub(crate) struct HighlightedSpans(pub Vec<HighlightedSpan>);
impl HighlightedSpans {
    pub(crate) fn apply_edit(self, edited_range: &Range<usize>, change: isize) -> HighlightedSpans {
        HighlightedSpans(
            self.0
                .into_iter()
                .filter_map(|span| span.apply_edit(edited_range, change))
                .collect(),
        )
    }
    pub(crate) fn apply_edit_mut(&mut self, edited_range: &Range<usize>, change: isize) {
        self.0
            .retain_mut(|span| span.apply_edit_mut(edited_range, change))
    }
}

pub(crate) struct SyntaxHighlightRequest {
    pub(crate) component_id: ComponentId,
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

    std::thread::spawn(move || {
        let mut highlight_configs = HighlightConfigs::new();
        let debounce = EventDebouncer::new(Duration::from_millis(150), move |Event(request)| {
            match highlight_configs.highlight(request.language, &request.source_code) {
                Ok(highlighted_spans) => {
                    let _ = callback.send(AppMessage::SyntaxHighlightResponse {
                        component_id: request.component_id,
                        highlighted_spans,
                    });
                }
                Err(error) => {
                    log::info!("syntax_highlight_error = {:#?}", error)
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
        config.highlight(source_code)
    }
}
