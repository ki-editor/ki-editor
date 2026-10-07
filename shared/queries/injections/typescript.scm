; Based on nvim-treesitter's `typescript` injections, on top of `ECMA_INJECTION_QUERY`.
; styled.div<{}>`...`
(call_expression
  function: (non_null_expression
    (instantiation_expression
      (member_expression
        object: (identifier) @_name
        property: (property_identifier))
      type_arguments: (type_arguments)))
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled.div<T>`...`
(binary_expression
  left: (binary_expression
    left: (member_expression
      object: (identifier) @_name
      property: (property_identifier))
    right: (identifier))
  right: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))
