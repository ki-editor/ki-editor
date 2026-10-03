; Based on nvim-treesitter's `rust` injections. Not ported: Rust in the arguments of other
; macros (the host highlights every token of a token tree, which tree-sitter-highlight lets win
; over the injected highlights), `html!` (needs `#offset!` to skip the braces), the left-hand
; side of `macro_rules!` (it is not Rust syntax), regex, re2c and comments.
; json!({ "a": 1 }): the JSON is the token tree inside the parentheses
((macro_invocation
  macro: [
    (scoped_identifier
      name: (_) @_macro_name)
    (identifier) @_macro_name
  ]
  (token_tree
    (token_tree) @injection.content))
  (#eq? @_macro_name "json")
  (#set! injection.language "json")
  (#set! injection.include-children))

; macro_rules! m { (...) => { ... }; }
((macro_definition
  (macro_rule
    right: (token_tree) @injection.content))
  (#set! injection.language "rust")
  (#set! injection.include-children))
