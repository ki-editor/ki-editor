; Based on nvim-treesitter's `elixir` injections. Not ported: surface (`~F`), eex (`~E`, `~L`),
; LiveView Native (`~LVN`), regex (`~r`) and comments.
; @moduledoc """...""", @doc "...", @typedoc ~S"..."
(unary_operator
  operator: "@"
  operand: (call
    target: (identifier) @_identifier
    (arguments
      [
        (string
          (quoted_content) @injection.content)
        (sigil
          (quoted_content) @injection.content)
      ]))
  (#any-of? @_identifier "moduledoc" "typedoc" "shortdoc" "doc")
  (#set! injection.language "markdown"))

; ~H"""...""" (HEEx)
((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
  (#eq? @_sigil_name "H")
  (#set! injection.language "heex"))

; ~z"..." (Zigler)
((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
  (#any-of? @_sigil_name "z" "Z")
  (#set! injection.language "zig"))

; ~j"..."
((sigil
  (sigil_name) @_sigil_name
  (quoted_content) @injection.content)
  (#any-of? @_sigil_name "j" "J")
  (#set! injection.language "json"))
