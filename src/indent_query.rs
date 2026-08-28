use std::collections::BTreeSet;

use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

use crate::{buffer::Buffer, selection::CharIndex};

/// Byte ranges of every node in a tree captured by a particular capture name.
type NodeRanges = Vec<(usize, usize)>;

/// Runs `query` over `tree` and returns the byte ranges of every node captured by `@indent`
/// and (if the query defines it) `@outdent`, so callers can test ancestors of a position for
/// membership. Shared by both `compute_indent_for_new_line` and
/// `compute_reindent_for_outdent_keyword`.
fn indent_and_outdent_ranges(
    query: &Query,
    tree: &Tree,
    source: &str,
) -> anyhow::Result<Option<(NodeRanges, NodeRanges)>> {
    let Some(indent_capture_index) = query.capture_index_for_name("indent") else {
        return Ok(None);
    };
    let outdent_capture_index = query.capture_index_for_name("outdent");

    let mut query_cursor = QueryCursor::new();
    let mut matches = query_cursor.matches(query, tree.root_node(), source.as_bytes());

    let mut indent_node_ranges: Vec<(usize, usize)> = Vec::new();
    let mut outdent_node_ranges: Vec<(usize, usize)> = Vec::new();
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let node = capture.node;
            let range = (node.start_byte(), node.end_byte());
            if capture.index == indent_capture_index {
                indent_node_ranges.push(range);
            } else if Some(capture.index) == outdent_capture_index {
                outdent_node_ranges.push(range);
            }
        }
    }
    Ok(Some((indent_node_ranges, outdent_node_ranges)))
}

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
///
/// `@outdent` is also honoured: when the token immediately *after* the
/// cursor (i.e. the token that is about to become the first thing on the
/// new line, since `enter_newline` inserts the indent right before it) is
/// itself `@outdent`-captured -- e.g. a closing `)`/`]`/`}` -- one level is
/// subtracted. This is what keeps a hanging continuation like
/// `foo(\n    <cursor>)` from leaving the pushed-down `)` over-indented.
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
    let source = buffer.rope().to_string();
    let Some((indent_node_ranges, outdent_node_ranges)) =
        indent_and_outdent_ranges(&query, tree, &source)?
    else {
        return Ok(None);
    };

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

    // The new line will begin with whatever token currently sits right after the cursor
    // (nothing is inserted there -- `enter_newline` only inserts `"\n" + indent` at the
    // cursor). If that token -- or an ancestor that starts at the very same byte, i.e. one
    // that this token is the *first* token of -- is `@outdent`-captured, drop one level.
    let is_outdent = {
        let mut current_node = tree.root_node().descendant_for_byte_range(byte, byte);
        let mut found = false;
        while let Some(node) = current_node {
            if node.start_byte() != byte {
                break;
            }
            if outdent_node_ranges.contains(&(node.start_byte(), node.end_byte())) {
                found = true;
                break;
            }
            current_node = node.parent();
        }
        found
    };

    let level = indent_rows.len().saturating_sub(is_outdent as usize);
    Ok(Some(
        std::iter::repeat_n(indent_char, indent_width)
            .collect::<String>()
            .repeat(level),
    ))
}

/// Computes the indentation an *already-typed* line should be corrected to, right after the
/// keystroke that completes a token captured `@outdent` in that position -- e.g. a closing
/// `)`/`]`/`}` typed by hand rather than pushed down by Enter, or finishing
/// `elif`/`else`/`except`/`finally`. Returns `None` when nothing should change.
///
/// This covers two genuinely different cases, tried in order:
///
/// - **Closing brackets** (`bracket_outdent_target`): tree-sitter parses these regardless of
///   the line's current (possibly wrong) indentation, so the fix can be read straight off the
///   real tree, the same way `compute_indent_for_new_line`'s outdent check does.
/// - **Clause keywords** (`keyword_outdent_target`): Python's grammar lexes indentation itself
///   (DEDENT/INDENT tokens), so e.g. `elif` only parses as part of an `elif_clause` once the
///   line is *already* dedented to align with its `if` -- which is exactly the correction this
///   function exists to make. The real tree can't be used to recognize the line while it is
///   still wrong, so this path matches the typed text directly against
///   `Language::outdent_keywords` instead.
///
/// Unlike `compute_indent_for_new_line`, this never runs on every keystroke blindly -- both
/// paths only ever fire on the keystroke that completes a trigger token, matching Helix's own
/// "reindent as you type these specific keywords" behaviour rather than reformatting arbitrary
/// lines as the user types unrelated content.
pub fn compute_reindent_for_outdent_keyword(
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
    let source = buffer.rope().to_string();
    let Some((indent_node_ranges, outdent_node_ranges)) =
        indent_and_outdent_ranges(&query, tree, &source)?
    else {
        return Ok(None);
    };

    // The token that begins the current line (skipping leading whitespace) is the only thing
    // ever eligible to trigger a reindent, and only once the user has finished typing it
    // exactly (nothing follows it on the line) -- so a still-incomplete prefix (`eli`) or
    // unrelated typing elsewhere on the line never touches this line's indentation.
    let current_line_index = buffer.char_to_line(cursor)?;
    let line_start = buffer.line_to_char(current_line_index)?;
    let line = buffer
        .get_line_by_line_index(current_line_index)
        .map(|line| line.to_string())
        .unwrap_or_default();
    let leading_whitespace_chars = line
        .chars()
        .take_while(|c| c.is_whitespace() && *c != '\n')
        .count();
    let content_start = line_start + leading_whitespace_chars;
    if cursor <= content_start {
        return Ok(None);
    }
    let typed: String = line
        .chars()
        .skip(leading_whitespace_chars)
        .take(cursor.0 - content_start.0)
        .collect();
    let rest_of_line: String = line
        .chars()
        .skip(leading_whitespace_chars + typed.chars().count())
        .collect();
    if !rest_of_line.trim_end_matches('\n').is_empty() {
        return Ok(None);
    }

    if let Some(target) = bracket_outdent_target(
        tree,
        &indent_node_ranges,
        &outdent_node_ranges,
        buffer.char_to_byte(content_start)?,
        buffer.char_to_byte(cursor)?,
        indent_char,
        indent_width,
    ) {
        return Ok(Some(target));
    }

    keyword_outdent_target(
        buffer,
        language.outdent_keywords(),
        &typed,
        current_line_index,
        line_start,
        indent_char,
        indent_width,
    )
}

/// Closing-bracket path of `compute_reindent_for_outdent_keyword`: walks up from the leaf at
/// `content_start_byte`, skipping every ancestor that -- like the leaf itself -- also starts
/// there (i.e. is still part of "this line's own opening"), noting along the way whether any of
/// them is `@outdent`-captured. Once an ancestor with an earlier start byte is reached, resumes
/// normal `@indent` counting from there, exactly as `compute_indent_for_new_line` does from a
/// plain cursor position -- this drops the bracket's own contribution to the count without
/// needing a separate subtraction step. Returns `None` when the token at `content_start_byte`
/// does not end exactly at `cursor_byte`, or is not itself `@outdent`-captured.
#[allow(clippy::too_many_arguments)]
fn bracket_outdent_target(
    tree: &Tree,
    indent_node_ranges: &[(usize, usize)],
    outdent_node_ranges: &[(usize, usize)],
    content_start_byte: usize,
    cursor_byte: usize,
    indent_char: char,
    indent_width: usize,
) -> Option<String> {
    let leaf = tree
        .root_node()
        .descendant_for_byte_range(content_start_byte, content_start_byte)?;
    if leaf.end_byte() != cursor_byte {
        return None;
    }

    let mut is_outdent = false;
    let mut current_node = Some(leaf);
    while let Some(node) = current_node {
        if node.start_byte() != content_start_byte {
            break;
        }
        if outdent_node_ranges.contains(&(node.start_byte(), node.end_byte())) {
            is_outdent = true;
        }
        current_node = node.parent();
    }
    if !is_outdent {
        return None;
    }

    let mut indent_rows = BTreeSet::new();
    while let Some(node) = current_node {
        if indent_node_ranges.contains(&(node.start_byte(), node.end_byte())) {
            indent_rows.insert(node.start_position().row);
        }
        current_node = node.parent();
    }

    Some(
        std::iter::repeat_n(indent_char, indent_width)
            .collect::<String>()
            .repeat(indent_rows.len()),
    )
}

/// Clause-keyword path of `compute_reindent_for_outdent_keyword`: fires when `typed` (the
/// current line's content up to the cursor, with nothing after it) exactly matches one of
/// `outdent_keywords`. The target is one level less than whatever indentation a brand new line
/// would get if Enter were pressed at the end of the *previous* line -- i.e. the level a normal
/// body statement continuing that block would get, minus one, which is where a clause keyword
/// belongs (aligned with the statement it is a clause of, not with that statement's body).
/// Reusing `compute_indent_for_new_line` this way relies only on the previous line, which --
/// unlike the current, still-wrongly-indented one -- is already validly parsed.
fn keyword_outdent_target(
    buffer: &Buffer,
    outdent_keywords: &[&str],
    typed: &str,
    current_line_index: usize,
    line_start: CharIndex,
    indent_char: char,
    indent_width: usize,
) -> anyhow::Result<Option<String>> {
    if current_line_index == 0 || !outdent_keywords.contains(&typed) {
        return Ok(None);
    }
    let one_level = std::iter::repeat_n(indent_char, indent_width).collect::<String>();
    let outer_indent =
        compute_indent_for_new_line(buffer, line_start - 1, indent_char, indent_width)?
            .unwrap_or_default();
    Ok(Some(
        outer_indent
            .strip_suffix(one_level.as_str())
            .unwrap_or("")
            .to_string(),
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
    fn indents_hanging_content_inside_an_open_bracket() -> anyhow::Result<()> {
        // The `(` is closed later on the same statement, so `argument_list` parses cleanly
        // and the cursor -- placed right after `(`, before `a` -- is inside it.
        let text = "foo(a)";
        let buffer = python_buffer(text);
        let cursor = CharIndex(4); // right after the `(`
        assert_eq!(
            compute_indent_for_new_line(&buffer, cursor, ' ', 4)?,
            Some("    ".to_string())
        );
        Ok(())
    }

    #[test]
    fn outdents_a_closing_bracket_pushed_onto_its_own_line() -> anyhow::Result<()> {
        // Cursor sits right after `(`, with the matching `)` immediately following it (as
        // it would after typing `foo()` and then moving the cursor back inside). Pressing
        // Enter here should push `)` down without indenting it, since it is closing the
        // bracket rather than continuing the hanging content.
        let text = "foo()";
        let buffer = python_buffer(text);
        let cursor = CharIndex(4); // right after the `(`
        assert_eq!(
            compute_indent_for_new_line(&buffer, cursor, ' ', 4)?,
            Some("".to_string())
        );
        Ok(())
    }

    #[test]
    fn outdent_does_not_cancel_an_indent_level_from_an_unrelated_ancestor() -> anyhow::Result<()> {
        // Nested one level inside `def foo():`, then a further hanging bracket -- the
        // outdent from the closing `)` should only cancel the bracket's own level, not
        // the `function_definition`'s.
        let text = "def foo():\n    bar()";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count() - 1); // right after the `(`
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

#[cfg(test)]
mod test_compute_reindent_for_outdent_keyword {
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
    fn outdents_elif_to_match_its_if() -> anyhow::Result<()> {
        // `elif` is still sitting at the same (wrong) column as `pass` above it -- tree-sitter
        // therefore can't recognize it as an `elif_clause` yet (Python's grammar requires it to
        // already be dedented), which is exactly the case `keyword_outdent_target` exists for.
        let text = "if x:\n    pass\n    elif";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count()); // right after the `f` of `elif`
        assert_eq!(
            compute_reindent_for_outdent_keyword(&buffer, cursor, ' ', 4)?,
            Some("".to_string())
        );
        Ok(())
    }

    #[test]
    fn outdents_a_second_except_relative_to_a_nested_try() -> anyhow::Result<()> {
        // A bare `try: pass` with no `except`/`finally` at all is not valid Python, and
        // tree-sitter-python's grammar reports large `ERROR` nodes for it that swallow even
        // the enclosing `def` (the same pre-existing grammar quirk Helix's own indents.scm
        // has a dedicated `ERROR`-node workaround for, which this POC does not port). So this
        // exercises a `try` that is already valid (has one `except` before the one being
        // typed) instead, keeping the surrounding tree -- and thus `function_definition`'s
        // and `try_statement`'s `@indent` levels -- intact.
        let text = "def foo():\n    try:\n        pass\n    except E:\n        pass\n    except";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(
            compute_reindent_for_outdent_keyword(&buffer, cursor, ' ', 4)?,
            Some("    ".to_string())
        );
        Ok(())
    }

    #[test]
    fn does_nothing_while_the_keyword_is_still_incomplete() -> anyhow::Result<()> {
        let text = "if x:\n    pass\n    eli";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(
            compute_reindent_for_outdent_keyword(&buffer, cursor, ' ', 4)?,
            None
        );
        Ok(())
    }

    #[test]
    fn does_nothing_for_a_plain_statement() -> anyhow::Result<()> {
        let text = "if x:\n    pass\n    y";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(
            compute_reindent_for_outdent_keyword(&buffer, cursor, ' ', 4)?,
            None
        );
        Ok(())
    }

    #[test]
    fn does_nothing_once_the_cursor_has_moved_past_the_keyword() -> anyhow::Result<()> {
        // A space was typed after `elif`; the keystroke that completed `elif` itself has
        // already been handled, so this one should not re-trigger.
        let text = "if x:\n    pass\n    elif ";
        let buffer = python_buffer(text);
        let cursor = CharIndex(text.chars().count());
        assert_eq!(
            compute_reindent_for_outdent_keyword(&buffer, cursor, ' ', 4)?,
            None
        );
        Ok(())
    }
}
