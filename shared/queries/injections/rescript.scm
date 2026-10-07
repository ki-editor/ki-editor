; Based on nvim-treesitter's `rescript` injections. Not ported: regex (`%re`) and comments.
; %raw("...") and %raw(`...`)
(extension_expression
  (extension_identifier) @_name
  (expression_statement
    [
      (string
        (string_fragment) @injection.content)
      (template_string
        (template_string_content) @injection.content)
    ])
  (#eq? @_name "raw")
  (#set! injection.language "javascript"))

; %graphql(`...`), %relay(`...`)
(extension_expression
  (extension_identifier) @_name
  (expression_statement
    [
      (string
        (string_fragment) @injection.content)
      (template_string
        (template_string_content) @injection.content)
    ])
  (#any-of? @_name "graphql" "relay")
  (#set! injection.language "graphql"))
