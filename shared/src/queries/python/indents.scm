; Indent query for Python, authored for ki-editor.
;
; This is a proof of concept: a hand-written replacement for the kind of
; `indents.scm` that editors like Helix ship per-language, now that
; nvim-treesitter (the previous source ki-editor vendors `highlights.scm`
; from) has been archived.
;
; Convention (same capture names as Helix's indent queries, kept because it
; is a well-documented, already-understood spec):
;   @indent    the node opens one extra level of indentation for every line
;              strictly inside it (excluding its own first line).
;   @outdent   when the token this is attached to is about to become the
;              first thing on a newly-inserted line, one enclosing @indent
;              level is subtracted for that line (see
;              `compute_indent_for_new_line`); when it is instead the token
;              that was *just typed* to complete an existing line, that
;              line's own indentation is corrected in place to what it
;              would be with that level subtracted (see
;              `compute_reindent_for_outdent_keyword`) -- this is what
;              re-aligns `elif`/`else`/`except`/`finally` to their opening
;              keyword as they are typed.
;
; Compound statements: everything that ends its header with `:` and owns a
; `block` of indented statements. `elif_clause`/`else_clause`/
; `except_clause`/`except_group_clause`/`finally_clause` are deliberately
; NOT included here even though they too own a `:`-terminated block: they
; are direct children of their `if_statement`/`try_statement`, at the same
; textual level as its own header, rather than nested inside it -- the
; parent statement's `@indent` already covers their entire span (headers
; and blocks alike), so also marking the clauses `@indent` would double-
; count and over-indent everything inside them.
[
  (if_statement)
  (for_statement)
  (while_statement)
  (try_statement)
  (with_statement)
  (function_definition)
  (class_definition)
  (match_statement)
  (case_clause)
] @indent

; Bracketed constructs: a hanging continuation right after an unclosed `(`,
; `[` or `{` gets one extra level, same as a `block`. Because the algorithm
; in `src/indent_query.rs` only ever counts *ancestors of the cursor*, a
; construct that is opened and closed on one line (`foo(a, b)`, cursor
; after it) is simply not an ancestor of the cursor and never contributes a
; level -- only a genuinely-open bracket the cursor sits inside does.
[
  (list)
  (tuple)
  (dictionary)
  (set)
  (parenthesized_expression)
  (generator_expression)
  (list_comprehension)
  (set_comprehension)
  (dictionary_comprehension)
  (tuple_pattern)
  (list_pattern)
  (argument_list)
  (parameters)
] @indent

; Closing brackets: drop back down to the level of the line that opened
; them, rather than inheriting the hanging-content level above.
[
  ")"
  "]"
  "}"
] @outdent

; Clause keywords: re-align to the level of the `if`/`try` they belong to
; (rather than the level of the block above them) once fully typed. These are matched by
; `Language::outdent_keywords`, not read off the tree directly the way the brackets above are
; -- Python's grammar lexes indentation itself, so e.g. `elif` only parses as part of an
; `elif_clause` once its line is *already* dedented to align with `if`, which is exactly the
; correction being computed. See `compute_reindent_for_outdent_keyword` in
; `src/indent_query.rs` for how these are actually used.
(elif_clause "elif" @outdent)
(else_clause "else" @outdent)
(except_clause "except" @outdent)
(finally_clause "finally" @outdent)
(except_group_clause "except*" @outdent)

; NOTE (known gap, not introduced by the above): a bare `try: ...` with no `except`/`finally`
; at all is not valid Python, and tree-sitter-python's grammar reports large `ERROR` nodes for
; it that can swallow even the enclosing `def`/`class` -- the same quirk Helix's own
; indents.scm has a handful of dedicated `ERROR`-node patterns to work around (using `@extend`,
; which this POC does not implement). While a `try` is missing its first `except`/`finally`,
; both indentation paths above can therefore under-indent. Typing the first `except`/`finally`
; itself is unaffected, since `keyword_outdent_target` only ever looks at the *previous* line.
