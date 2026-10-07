; Based on nvim-treesitter's `php_only` and `php` injections. Not ported: phpdoc, regex
; (`preg_*`), heredoc/nowdoc (the language is the case-sensitive label) and
; `shell_exec("...")` and friends (the host highlights the string content, which
; tree-sitter-highlight lets win over the injected highlights).
; Inline HTML outside of <?php ... ?>
((text) @injection.content
  (#set! injection.language "html")
  (#set! injection.combined))

; `ls -la`
((shell_command_expression
  (string_content) @injection.content)
  (#set! injection.language "bash"))
