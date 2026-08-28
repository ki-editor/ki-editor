use std::collections::BTreeSet;

use tree_sitter::{Query, QueryCursor, StreamingIterator};

use crate::{buffer::Buffer, selection::CharIndex};

/// POC: computes the indentation string a new line should get when the user
/// presses Enter, by running the language's `indents.scm` query (see
/// `Language::indent_query`) against the buffer's tree.
///
/// Returns `None` when the language does not supply an `indents.scm` (or the
/// query/tree otherwise is not available) -- callers should fall back to the
/// default heuristic of copying the current line's own indentation in that
/// case. When `Some`, the returned string is the *complete* indentation for
/// the new line, computed from the syntax tree alone -- the default
/// copy-the-previous-line's-indentation heuristic is not consulted at all,
/// since a hand-authored `indents.scm` is assumed to know better than
/// whatever whitespace happens to precede the cursor.
///
/// Algorithm (a simplified version of Helix's): find the smallest node
/// covering the cursor, then walk up through its ancestors counting one
/// indent level for every distinct source line on which an `@indent`-
/// captured ancestor begins (several `@indent` scopes opening on the same
/// physical line only ever count once, matching Helix's convention).
pub fn compute_indent_for_new_line(
    buffer: &Buffer,
    cursor: CharIndex,
    indent_char: char,
    indent_width: usize,
) -> anyhow::Result<Option<String>> {
    let Some(language) = buffer.language() else {
        return Ok(None);
    };
    let Some(query_source) = language.indent_query() else {
        return Ok(None);
    };
    let Some(ts_language) = buffer.treesitter_language() else {
        return Ok(None);
    };
    let Some(tree) = buffer.tree() else {
        return Ok(None);
    };

    let query = Query::new(&ts_language, &query_source)?;
    let Some(indent_capture_index) = query.capture_index_for_name("indent") else {
        return Ok(None);
    };

    let source = buffer.rope().to_string();
    let mut query_cursor = QueryCursor::new();
    let mut matches = query_cursor.matches(&query, tree.root_node(), source.as_bytes());

    // Identify every node captured by `@indent` by its byte range, so ancestors of the
    // cursor can be tested for membership below.
    let mut indent_node_ranges: Vec<(usize, usize)> = Vec::new();
    while let Some(m) = matches.next() {
        for capture in m.captures {
            if capture.index == indent_capture_index {
                let node = capture.node;
                indent_node_ranges.push((node.start_byte(), node.end_byte()));
            }
        }
    }

    // Look at the node covering the character immediately *before* the cursor rather than
    // the cursor's own (zero-width) position: `descendant_for_byte_range` does not descend
    // into the empty range past the end of the last token (e.g. right after `if x:` at
    // the end of the buffer, it would resolve all the way up to the root), whereas biasing
    // one byte to the left lands inside whatever was just typed.
    let byte = buffer.char_to_byte(cursor)?;
    let lookup_byte = byte.saturating_sub(1);
    let mut current_node = tree
        .root_node()
        .descendant_for_byte_range(lookup_byte, lookup_byte);
    let mut indent_rows = BTreeSet::new();
    while let Some(node) = current_node {
        if indent_node_ranges.contains(&(node.start_byte(), node.end_byte())) {
            indent_rows.insert(node.start_position().row);
        }
        current_node = node.parent();
    }

    let level = indent_rows.len();
    Ok(Some(
        std::iter::repeat_n(indent_char, indent_width)
            .collect::<String>()
            .repeat(level),
    ))
}

#[cfg(test)]
mod test_compute_indent_for_new_line {
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
    fn indents_one_level_after_compound_statement_header() -> anyhow::Result<()> {
        let text = "if x:";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(
            compute_indent_for_new_line(&buffer, cursor, ' ', 4)?,
            Some("    ".to_string())
        );
        Ok(())
    }

    #[test]
    fn indents_two_levels_when_nested() -> anyhow::Result<()> {
        let text = "def foo():\n    if x:";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(
            compute_indent_for_new_line(&buffer, cursor, ' ', 4)?,
            Some("        ".to_string())
        );
        Ok(())
    }

    #[test]
    fn no_extra_level_on_a_plain_statement() -> anyhow::Result<()> {
        let text = "def foo():\n    x = 1";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(
            compute_indent_for_new_line(&buffer, cursor, ' ', 4)?,
            Some("    ".to_string())
        );
        Ok(())
    }

    #[test]
    fn non_python_language_is_unaffected() -> anyhow::Result<()> {
        // Rust's `Language::indent_query` is not implemented in this POC, so callers
        // should fall back to the default heuristic in that case.
        let language = shared::languages::languages().get("rust").unwrap().clone();
        let ts_language = language.tree_sitter_language().unwrap();
        let text = "fn foo() {";
        let mut buffer = Buffer::new(Some(ts_language), text);
        buffer.set_language(language)?;
        let cursor = CharIndex(text.chars().count());
        assert_eq!(compute_indent_for_new_line(&buffer, cursor, ' ', 4)?, None);
        Ok(())
    }
}
