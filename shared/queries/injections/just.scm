; Based on nvim-treesitter's `just` injections. The shebang language is restricted to
; languages that are known to Ki. Not ported: bash for recipes without a shebang (the host
; highlights the recipe line as a string, which tree-sitter-highlight lets win over the injected
; highlight of the first token), regex (`=~`) and comments.
; `ls`
((external_command
  (command_body) @injection.content)
  (#set! injection.language "bash"))

; For shebang recipes, use the shebang executable name as the language
((recipe
  (recipe_body
    (shebang
      (language) @injection.language)) @injection.content)
  (#any-of? @injection.language "bash" "zsh" "fish" "python" "perl" "ruby" "lua")
  (#set! injection.include-children))

; sh -> bash
((recipe
  (recipe_body
    (shebang
      (language) @_lang)) @injection.content)
  (#eq? @_lang "sh")
  (#set! injection.language "bash")
  (#set! injection.include-children))

; python3 -> python
((recipe
  (recipe_body
    (shebang
      (language) @_lang)) @injection.content)
  (#eq? @_lang "python3")
  (#set! injection.language "python")
  (#set! injection.include-children))

; node/nodejs -> javascript
((recipe
  (recipe_body
    (shebang
      (language) @_lang)) @injection.content)
  (#any-of? @_lang "node" "nodejs")
  (#set! injection.language "javascript")
  (#set! injection.include-children))
