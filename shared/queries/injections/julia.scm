; Based on nvim-treesitter's `julia` injections. Not ported: regex (`r"..."`) and comments.
; Docstrings
((string_literal
  (content) @injection.content)
  .
  [
    (module_definition)
    (abstract_definition)
    (struct_definition)
    (function_definition)
    (macro_definition)
    (assignment)
    (const_statement)
    (call_expression)
    (identifier)
  ]
  (#set! injection.language "markdown"))

; md"**Bold** and _Italics_" and md"""..."""
((prefixed_string_literal
  prefix: (identifier) @_prefix
  (content) @injection.content)
  (#eq? @_prefix "md")
  (#set! injection.language "markdown"))

; `git add --help`
((command_literal
  (content) @injection.content)
  (#set! injection.language "bash"))
