; Based on `tree_sitter_md::INJECTION_QUERY_BLOCK`, except that
; `injection.include-children` is set on every rule: block nodes have one
; `block_continuation` child per line, which would otherwise be excluded from the
; injected ranges and split the embedded code into unparsable fragments.
(fenced_code_block
  (info_string
    (language) @injection.language)
  (code_fence_content) @injection.content
  (#set! injection.include-children))

((html_block) @injection.content
  (#set! injection.language "html")
  (#set! injection.include-children))

(document . (section . (thematic_break) (_) @injection.content (thematic_break))
  (#set! injection.language "yaml")
  (#set! injection.include-children))

([(minus_metadata) (plus_metadata)] @injection.content
  (#set! injection.language "yaml")
  (#set! injection.include-children))
