; Based on nvim-treesitter's `xml` injections, except that `injection.combined` is not set so
; that every element is parsed on its own.
; <style> and <script> (e.g. in SVG). Children are included because the character
; data of an element is a child of its `content` node.
((element
  (STag
    (Name) @_name)
  (content) @injection.content)
  (#eq? @_name "style")
  (#set! injection.include-children)
  (#set! injection.language "css"))

((element
  (STag
    (Name) @_name)
  (content) @injection.content)
  (#eq? @_name "script")
  (#set! injection.include-children)
  (#set! injection.language "javascript"))

; phpMyAdmin dump
((element
  (STag
    (Name) @_name)
  (content) @injection.content)
  (#eq? @_name "pma:table")
  (#set! injection.include-children)
  (#set! injection.language "sql"))
