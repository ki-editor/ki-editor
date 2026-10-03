; Based on nvim-treesitter's `yaml` injections. Not ported: plain scalars such as
; `run: echo hi` (the host highlights the scalar as a string, which tree-sitter-highlight lets
; win over the injected highlight of the first token), Prometheus `expr` (promql) and comments.
; GitHub Actions ("run"), GitLab CI ("script"), Taskfile ("cmds", "cmd", "sh"). The block
; scalar indicator (`|`, `>`) is part of the captured node, so the shell sees it as a stray
; token at the start of the script.
((block_mapping_pair
  key: (flow_node) @_run
  value: (block_node
    (block_scalar) @injection.content))
  (#any-of? @_run "run" "script" "before_script" "after_script" "cmds" "cmd" "sh")
  (#set! injection.language "bash"))

((block_mapping_pair
  key: (flow_node) @_run
  value: (block_node
    (block_sequence
      (block_sequence_item
        (block_node
          (block_scalar) @injection.content)))))
  (#any-of? @_run "script" "before_script" "after_script" "cmds" "sh")
  (#set! injection.language "bash"))
