; Based on nvim-treesitter's `jsx` injections.
; <style jsx>{`...`}</style>
(jsx_element
  (jsx_opening_element
    (identifier) @_name
    (jsx_attribute) @_attr)
  (jsx_expression
    (template_string
      (string_fragment) @injection.content))
  (#eq? @_name "style")
  (#eq? @_attr "jsx")
  (#set! injection.language "css")
  (#set! injection.combined))
