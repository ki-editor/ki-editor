; Indent query for Python, authored for ki-editor.
;
; This is a proof of concept: a hand-written replacement for the kind of
; `indents.scm` that editors like Helix ship per-language, now that
; nvim-treesitter (the previous source ki-editor vendors `highlights.scm`
; from) has been archived.
;
; Convention (same capture names as Helix's indent queries, kept because it
; is a well-documented, already-understood spec):
;   @indent   the node opens one extra level of indentation for every line
;             strictly inside it (excluding its own first line).
;
; Only `@indent` is implemented for this POC. `@outdent` (e.g. aligning
; `elif`/`except`/`else` back to their opening keyword when they are typed)
; is intentionally left out -- it needs to run on keystrokes other than
; Enter, which is a separate feature from what issue #525 asks for.

; Compound statements: everything that ends its header with `:` and owns a
; `block` of indented statements.
[
  (if_statement)
  (elif_clause)
  (else_clause)
  (for_statement)
  (while_statement)
  (try_statement)
  (except_clause)
  (except_group_clause)
  (finally_clause)
  (with_statement)
  (function_definition)
  (class_definition)
  (match_statement)
  (case_clause)
] @indent

; NOTE (POC follow-up): bracketed constructs -- e.g. indenting a hanging
; continuation right after an unclosed `(`, `[` or `{` -- are deliberately
; NOT covered yet. Telling "still open" apart from "closed on this same
; line" (`foo(a, b)` should NOT indent) requires checking whether the
; node's last child is actually the closing token, which the interpreter
; in `src/indent_query.rs` does not do yet. Adding it is the natural next
; step once the compound-statement case here is validated.
