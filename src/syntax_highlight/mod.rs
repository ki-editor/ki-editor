use std::ops::Range;

use tree_sitter_highlight::{HighlightEvent, Highlighter};

use crate::{grid::Style, language::Language, selection::CharIndex, themes::Theme};

#[derive(Clone)]
pub struct HighlighedSpan {
    pub range: Range<CharIndex>,
    pub style: Style,
}

/// In hex format, e.g. "#FF0000"
#[derive(Debug, Clone, Copy)]
pub struct Color(&'static str);

pub fn highlight(
    language: Box<dyn Language>,
    theme: &Theme,
    source_code: &str,
) -> anyhow::Result<Vec<HighlighedSpan>> {
    let tree_sitter_language = if let Some(tree_sitter_language) = language.tree_sitter_language() {
        tree_sitter_language
    } else {
        return Ok(vec![]);
    };
    let mut highlighter = Highlighter::new();
    use tree_sitter_highlight::HighlightConfiguration;

    let highlight_names = &[
        "attribute",
        "constant",
        "function.builtin",
        "function",
        "keyword",
        "operator",
        "property",
        "punctuation",
        "punctuation.bracket",
        "punctuation.delimiter",
        "string",
        "string.special",
        "tag",
        "type",
        "type.builtin",
        "variable",
        "variable.builtin",
        "variable.parameter",
    ];

    let mut config = HighlightConfiguration::new(
        tree_sitter_language,
        language.highlight_query().unwrap_or_default(),
        language.injection_query().unwrap_or_default(),
        language.locals_query().unwrap_or_default(),
    )
    .unwrap();

    config.configure(highlight_names);

    let highlights = highlighter
        .highlight(&config, source_code.as_bytes(), None, |_| None)
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
                    if let Some(color) = theme.styles.get_color(highlight.0) {
                        highlighted_spans.push(HighlighedSpan {
                            range: CharIndex(start)..CharIndex(end),
                            style: color,
                        });
                    }
                }
            }
        }
    }
    Ok(highlighted_spans)
}
