; Based on nvim-treesitter's `gitcommit` injections. Not ported: `rebase_command` (git_rebase),
; which the grammar only produces in rebase todo lists.
((diff) @injection.content
  (#set! injection.language "diff"))
