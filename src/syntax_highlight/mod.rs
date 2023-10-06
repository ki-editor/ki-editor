use std::ops::Range;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{
    components::component::ComponentId,
    grid::{Style, StyleSource},
    themes::Theme,
};
use shared::language::Language;

#[derive(Clone, Debug)]
pub struct HighlighedSpan {
    pub byte_range: Range<usize>,
    pub style: Style,
    pub source: Option<StyleSource>,
}

/// In hex format, e.g. "#FF0000"
#[derive(Debug, Clone, Copy)]
pub struct Color(&'static str);

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

        let mut config = HighlightConfiguration::new(
            tree_sitter_language,
            &self.highlight_query().unwrap_or_default(),
            self.injection_query().unwrap_or_default(),
            self.locals_query().unwrap_or_default(),
        )?;

        config.configure(crate::themes::HIGHLIGHT_NAMES);

        Ok(Some(config))
    }
}

pub trait Highlight {
    fn highlight(&self, theme: &Theme, source_code: &str) -> anyhow::Result<HighlighedSpans>;
}
impl Highlight for HighlightConfiguration {
    fn highlight(&self, theme: &Theme, source_code: &str) -> anyhow::Result<HighlighedSpans> {
        let mut highlighter = Highlighter::new();

        let highlights = highlighter
            .highlight(&self, source_code.as_bytes(), None, |_| None)
            .unwrap();

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
                        if let Some(color) = theme.syntax.get_color(highlight.0) {
                            highlighted_spans.push(HighlighedSpan {
                                byte_range: start..end,
                                style: color,
                                source: match crate::themes::HIGHLIGHT_NAMES.get(highlight.0) {
                                    Some(&"comment") => Some(StyleSource::SyntaxComment),
                                    Some(&"keyword") => Some(StyleSource::SyntaxKeyword),
                                    Some(&"string") => Some(StyleSource::SyntaxString),
                                    Some(&"type") => Some(StyleSource::SyntaxType),
                                    Some(&"function") => Some(StyleSource::SyntaxFunction),
                                    _ => None,
                                },
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

pub struct SyntaxHighlightRequest {
    pub component_id: ComponentId,
    pub language: Language,
    pub theme: Theme,
    pub source_code: String,
}

pub struct SyntaxHighlightResponse {
    pub component_id: ComponentId,
    pub highlighted_spans: HighlighedSpans,
}
