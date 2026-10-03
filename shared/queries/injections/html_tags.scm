; Based on nvim-treesitter's `html_tags` injections, shared by languages that embed HTML
; elements. `#lua-match?` and `#gsub!` are rewritten as `#match?` and explicit `type` values.
; Not ported: `style="..."` (a declaration list is not a stylesheet), `on*="..."` handlers
; (the attribute value is highlighted as a string by the host, which tree-sitter-highlight
; lets win over the injected highlights at the start of the range), lit-html `${}`
; attributes (needs `#offset!`), `pattern="..."` (no regex language), and comments.
; <style>...</style>; `lang`/`type` attributes are handled by the rules below
((style_element
  (start_tag) @_start_tag
  (raw_text) @injection.content)
  (#not-match? @_start_tag "\\s(lang|type)\\s*=")
  (#set! injection.language "css"))

((style_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#eq? @_type "text/css")
  (#set! injection.language "css"))

; <script>...</script>
((script_element
  (start_tag) @_start_tag
  (raw_text) @injection.content)
  (#not-match? @_start_tag "\\s(lang|type)\\s*=")
  (#set! injection.language "javascript"))

; <script type="module">, <script type="text/javascript">
((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#any-of? @_type "module" "text/javascript" "application/javascript" "text/ecmascript" "application/ecmascript")
  (#set! injection.language "javascript"))

((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#any-of? @_type "text/typescript" "application/typescript")
  (#set! injection.language "typescript"))

; <script type="importmap">, <script type="application/json">
((script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @_type)))
  (raw_text) @injection.content)
  (#eq? @_attr "type")
  (#any-of? @_type "importmap" "application/json")
  (#set! injection.language "json"))
