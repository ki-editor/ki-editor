; Based on nvim-treesitter's `heex` injections. Not ported: comments.
; Directives are standalone tags like `<%= @x %>`. The partial and ending expression values
; are fragments of one Elixir expression that spans multiple directives, e.g.
;     <%= if true do %>
;       <p>, tree-sitter!</p>
;     <% end %>
; so they are combined.
(directive
  [
    (partial_expression_value)
    (ending_expression_value)
  ] @injection.content
  (#set! injection.language "elixir")
  (#set! injection.include-children)
  (#set! injection.combined))

((directive
  (expression_value) @injection.content)
  (#set! injection.language "elixir"))

; <link href={ Routes.static_path(..) } />
((expression
  (expression_value) @injection.content)
  (#set! injection.language "elixir"))
