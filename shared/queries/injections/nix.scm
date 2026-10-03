; Based on nvim-treesitter's `nix` injections. `#lua-match?` is rewritten as `#match?`, and the
; `pre*`/`post*` hooks require a capital letter after the prefix so that attributes such as
; `prefix` are not treated as shell. Not ported: language comments such as `/* lua */` (needs
; `#gsub!`), regex and comments.
; Build phases: buildPhase, preInstall, postFixup, script, ...
((binding
  attrpath: (attrpath
    (identifier) @_path)
  expression: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_path "^([a-zA-Z]+Phase|(pre|post)[A-Z][a-zA-Z]*|script)$")
  (#set! injection.language "bash"))

; pkgs.writeShellApplication { text = ''...''; }
((apply_expression
  function: (_) @_func
  argument: (attrset_expression
    (binding_set
      (binding
        attrpath: (attrpath
          (identifier) @_path)
        expression: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ]))))
  (#match? @_func "(^|\\.)writeShellApplication$")
  (#eq? @_path "text")
  (#set! injection.language "bash")
  (#set! injection.combined))

; pkgs.runCommand "name" { } ''...''
((apply_expression
  function: (apply_expression
    function: (apply_expression
      function: (_) @_func))
  argument: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_func "(^|\\.)runCommand[a-zA-Z]*$")
  (#set! injection.language "bash")
  (#set! injection.combined))

; pkgs.writeBash "name" ''...'' (also writeDash and writeShellScript)
((apply_expression
  function: (apply_expression
    function: (_) @_func)
  argument: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_func "(^|\\.)write(Bash|Dash|ShellScript)[a-zA-Z]*$")
  (#set! injection.language "bash")
  (#set! injection.combined))

; pkgs.writeFish "name" ''...''
((apply_expression
  function: (apply_expression
    function: (_) @_func)
  argument: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_func "(^|\\.)writeFish[a-zA-Z]*$")
  (#set! injection.language "fish")
  (#set! injection.combined))

; pkgs.writeJS "name" ''...'' or pkgs.writeJS "name" { } ''...'' (likewise for the other
; interpreters below, which take optional arguments)
((apply_expression
  function: [
    (apply_expression
      function: (_) @_func)
    (apply_expression
      function: (apply_expression
        function: (_) @_func))
  ]
  argument: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_func "(^|\\.)writeJS[a-zA-Z]*$")
  (#set! injection.language "javascript")
  (#set! injection.combined))

; pkgs.writePerl "name" ''...''
((apply_expression
  function: [
    (apply_expression
      function: (_) @_func)
    (apply_expression
      function: (apply_expression
        function: (_) @_func))
  ]
  argument: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_func "(^|\\.)writePerl[a-zA-Z]*$")
  (#set! injection.language "perl")
  (#set! injection.combined))

; pkgs.writePy "name" ''...''
((apply_expression
  function: [
    (apply_expression
      function: (_) @_func)
    (apply_expression
      function: (apply_expression
        function: (_) @_func))
  ]
  argument: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_func "(^|\\.)writePy[a-zA-Z]*[0-9]*[a-zA-Z]*$")
  (#set! injection.language "python")
  (#set! injection.combined))

; pkgs.writeRust "name" ''...''
((apply_expression
  function: [
    (apply_expression
      function: (_) @_func)
    (apply_expression
      function: (apply_expression
        function: (_) @_func))
  ]
  argument: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_func "(^|\\.)writeRust[a-zA-Z]*$")
  (#set! injection.language "rust")
  (#set! injection.combined))

; pkgs.writeHaskell "name" { } ''...''
((apply_expression
  function: [
    (apply_expression
      function: (_) @_func)
    (apply_expression
      function: (apply_expression
        function: (_) @_func))
  ]
  argument: [
    (string_expression
      (string_fragment) @injection.content)
    (indented_string_expression
      (string_fragment) @injection.content)
  ])
  (#match? @_func "(^|\\.)writeHaskell[a-zA-Z]*$")
  (#set! injection.language "haskell")
  (#set! injection.combined))

; testScript of (runNixOS)Test
((apply_expression
  function: (_) @_func
  argument: (attrset_expression
    (binding_set
      (binding
        attrpath: (attrpath) @_func_name
        expression: (_
          (string_fragment) @injection.content)))))
  (#eq? @_func_name "testScript")
  (#match? @_func "(^|\\.)(runTest|nixosTest|runNixOSTest)$")
  (#set! injection.language "python")
  (#set! injection.combined))

; home-manager Neovim plugin config: { type = "lua"; config = ''...''; }
((attrset_expression
  (binding_set
    (binding
      attrpath: (attrpath) @_ty_attr
      expression: (_
        (string_fragment) @_ty))
    (binding
      attrpath: (attrpath) @_cfg_attr
      expression: (_
        (string_fragment) @injection.content))))
  (#eq? @_ty_attr "type")
  (#eq? @_ty "lua")
  (#eq? @_cfg_attr "config")
  (#set! injection.language "lua")
  (#set! injection.combined))
