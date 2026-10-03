; Based on nvim-treesitter's `lua` injections. Only `ffi.cdef` (C) is ported: the others
; inject vimscript, tree-sitter queries, luap, luadoc, printf or comments.
; ffi.cdef([[ int f(int x); ]])
((function_call
  name: [
    (identifier) @_cdef_identifier
    (dot_index_expression
      field: (identifier) @_cdef_identifier)
  ]
  arguments: (arguments
    (string
      content: (string_content) @injection.content)))
  (#eq? @_cdef_identifier "cdef")
  (#set! injection.language "c"))
