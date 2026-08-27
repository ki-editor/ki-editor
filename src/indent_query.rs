use tree_sitter::{Query, QueryCursor, StreamingIterator};

use crate::{buffer::Buffer, selection::CharIndex};

/// POC: computes how many extra indent levels should be added on top of the
/// current line's own indentation when the user presses Enter, by running
/// the language's `indents.scm` query (see `Language::indent_query`) and
/// checking whether the line being left opens one or more `@indent` scopes
/// that are still unclosed at the cursor.
///
/// Only the "does this line open a new scope" question is answered here
/// (deliberately not the full Helix-style "count every enclosing scope"
/// algorithm), because the existing caller already copies the current
/// line's own indentation verbatim -- we only need to know whether to add
/// *one more* level on top of that.
pub fn compute_extra_indent_level(buffer: &Buffer, cursor: CharIndex) -> anyhow::Result<usize> {
    let Some(language) = buffer.language() else {
        return Ok(0);
    };
    let Some(query_source) = language.indent_query() else {
        return Ok(0);
    };
    let Some(ts_language) = buffer.treesitter_language() else {
        return Ok(0);
    };
    let Some(tree) = buffer.tree() else {
        return Ok(0);
    };
    let current_line = buffer.char_to_line(cursor)?;

    let query = Query::new(&ts_language, &query_source)?;
    let Some(indent_capture_index) = query.capture_index_for_name("indent") else {
        return Ok(0);
    };

    let source = buffer.rope().to_string();
    let mut query_cursor = QueryCursor::new();
    let mut matches = query_cursor.matches(&query, tree.root_node(), source.as_bytes());

    let mut opens_new_scope = false;
    while let Some(m) = matches.next() {
        for capture in m.captures {
            if capture.index != indent_capture_index {
                continue;
            }
            let node = capture.node;
            let start_row = node.start_position().row;
            // The "same-line" rule, borrowed from Helix's indent
            // algorithm: several scopes opening on one physical line only
            // ever add one level. We only care whether a scope *begins* on
            // the line being left.
            //
            // Note this is deliberately simple and has a known false
            // positive: a single-line compound statement whose body is
            // already present on the same line (e.g. `if x: pass`) will
            // still be counted as "opening a scope" here, since
            // tree-sitter-python's error recovery does not reliably let us
            // tell "body already closed on this line" apart from "body not
            // written yet" -- see indent_query.rs POC notes.
            if start_row == current_line {
                opens_new_scope = true;
            }
        }
    }

    Ok(if opens_new_scope { 1 } else { 0 })
}

#[cfg(test)]
mod test_compute_extra_indent_level {
    use super::*;

    fn python_buffer(text: &str) -> Buffer {
        let language = shared::languages::languages()
            .get("python")
            .unwrap()
            .clone();
        let ts_language = language.tree_sitter_language().unwrap();
        let mut buffer = Buffer::new(Some(ts_language), text);
        buffer.set_language(language).unwrap();
        buffer
    }

    #[test]
    fn opens_extra_level_after_compound_statement_header() -> anyhow::Result<()> {
        let text = "def foo():\n    if x:";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(compute_extra_indent_level(&buffer, cursor)?, 1);
        Ok(())
    }

    #[test]
    fn no_extra_level_on_a_plain_statement() -> anyhow::Result<()> {
        let text = "def foo():\n    x = 1";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(compute_extra_indent_level(&buffer, cursor)?, 0);
        Ok(())
    }

    #[test]
    fn non_python_language_is_unaffected() -> anyhow::Result<()> {
        // Rust's `Language::indent_query` is not implemented in this POC,
        // so it should always fall back to 0 extra levels.
        let language = shared::languages::languages().get("rust").unwrap().clone();
        let ts_language = language.tree_sitter_language().unwrap();
        let text = "fn foo() {";
        let mut buffer = Buffer::new(Some(ts_language), text);
        buffer.set_language(language)?;
        let cursor = CharIndex(text.chars().count());
        assert_eq!(compute_extra_indent_level(&buffer, cursor)?, 0);
        Ok(())
    }
}
