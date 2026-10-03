; Based on nvim-treesitter's `dockerfile` injections. Not ported: `RUN <<EOF` heredocs (the
; host highlights each heredoc line as a string, which tree-sitter-highlight lets win over the
; injected highlights at the start of each line) and comments.
; RUN, CMD and ENTRYPOINT in shell form. A command continued over multiple lines has one
; fragment per line, which are combined into one script.
((shell_command
  (shell_fragment) @injection.content)
  (#set! injection.language "bash")
  (#set! injection.combined))
