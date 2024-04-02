use std::{
    collections::HashMap,
    ops::Range,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use tokio::sync::mpsc::UnboundedSender;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{
    app::AppMessage, char_index_range::apply_edit, components::component::ComponentId,
    edit::is_overlapping, grid::StyleKey, themes::HIGHLIGHT_NAMES,
};
use shared::language::Language;

#[derive(Clone, Debug)]
pub struct HighlighedSpan {
    pub byte_range: Range<usize>,
    pub style_key: StyleKey,
}
impl HighlighedSpan {
    fn apply_edit(self, edited_range: &Range<usize>, change: isize) -> Option<HighlighedSpan> {
        Some(HighlighedSpan {
            byte_range: apply_edit(self.byte_range, edited_range, change)?,
            ..self
        })
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

        config.configure(crate::themes::HIGHLIGHT_NAMES);

        Ok(Some(config))
    }
}

pub trait Highlight {
    fn highlight(&self, source_code: &str) -> anyhow::Result<HighlighedSpans>;
}

impl Highlight for HighlightConfiguration {
    fn highlight(&self, source_code: &str) -> anyhow::Result<HighlighedSpans> {
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
                        if let Some(style_key) = HIGHLIGHT_NAMES.get(highlight.0) {
                            let style_key = StyleKey::Syntax(style_key.to_string());
                            highlighted_spans.push(HighlighedSpan {
                                byte_range: start..end,
                                style_key,
                            });
                        }
                    }
                }
            }
        }
        Ok(HighlighedSpans(highlighted_spans))
    }
}

#[derive(Clone, Default, Debug)]
pub struct HighlighedSpans(pub Vec<HighlighedSpan>);
impl HighlighedSpans {
    pub(crate) fn apply_edit(self, edited_range: &Range<usize>, change: isize) -> HighlighedSpans {
        HighlighedSpans(
            self.0
                .into_iter()
                .filter_map(|span| span.apply_edit(edited_range, change))
                .collect(),
        )
    }
}

pub struct SyntaxHighlightRequest {
    pub component_id: ComponentId,
    pub language: Language,
    pub source_code: String,
}

pub struct SyntaxHighlightResponse {
    pub component_id: ComponentId,
    pub highlighted_spans: HighlighedSpans,
}

pub fn start_thread(callback: UnboundedSender<AppMessage>) -> Sender<SyntaxHighlightRequest> {
    let (sender, mut receiver) = std::sync::mpsc::channel::<SyntaxHighlightRequest>();
    use debounce::EventDebouncer;
    struct Event(SyntaxHighlightRequest);
    impl PartialEq for Event {
        fn eq(&self, other: &Self) -> bool {
            // Always return true, because every syntax highlight request should be debounced
            true
        }
    }

    std::thread::spawn(move || {
        let mut highlight_configs = HighlightConfigs::new();
        let debounce = EventDebouncer::new(Duration::from_millis(300), move |Event(request)| {
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
pub struct HighlightConfigs(
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
    ) -> Result<HighlighedSpans, anyhow::Error> {
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
