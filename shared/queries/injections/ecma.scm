; Based on nvim-treesitter's `ecma` injections. Template literals are captured through their
; `string_fragment` children, which sidesteps `#offset!` (unsupported) for the backticks.
; CSS-in-JS is parsed as plain CSS rather than nvim-treesitter's `styled`. Not ported: jsdoc,
; regex, groq, glimmer and angular.
; html`...`, html(`...`), sql`...`, graphql`...`; template substitutions are
; skipped and the remaining fragments are parsed as one document
(call_expression
  function: (identifier) @injection.language
  arguments: [
    (arguments
      (template_string
        (string_fragment) @injection.content))
    (template_string
      (string_fragment) @injection.content)
  ]
  (#any-of? @injection.language "html" "sql" "graphql")
  (#set! injection.combined))

; svg`...` or svg(`...`)
(call_expression
  function: (identifier) @_name
  arguments: [
    (arguments
      (template_string
        (string_fragment) @injection.content))
    (template_string
      (string_fragment) @injection.content)
  ]
  (#eq? @_name "svg")
  (#set! injection.language "html")
  (#set! injection.combined))

; gql`...`
(call_expression
  function: (identifier) @_name
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "gql")
  (#set! injection.language "graphql")
  (#set! injection.combined))

; foo.sql`...` or foo.sql(`...`)
(call_expression
  function: (member_expression
    property: (property_identifier) @_name)
  arguments: [
    (arguments
      (template_string
        (string_fragment) @injection.content))
    (template_string
      (string_fragment) @injection.content)
  ]
  (#eq? @_name "sql")
  (#set! injection.language "sql")
  (#set! injection.combined))

; /* tagged by a leading #graphql comment */
((template_string
  (string_fragment) @injection.content)
  (#match? @injection.content "^#graphql")
  (#set! injection.language "graphql"))

; css`...`, keyframes`...`
(call_expression
  function: (identifier) @_name
  arguments: (template_string
    (string_fragment) @injection.content)
  (#any-of? @_name "css" "keyframes")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled.div`...`
(call_expression
  function: (member_expression
    object: (identifier) @_name)
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled(Component)`...`
(call_expression
  function: (call_expression
    function: (identifier) @_name)
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled.div.attrs({ prop: "foo" })`...`
(call_expression
  function: (call_expression
    function: (member_expression
      object: (member_expression
        object: (identifier) @_name)))
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; styled(Component).attrs({ prop: "foo" })`...`
(call_expression
  function: (call_expression
    function: (member_expression
      object: (call_expression
        function: (identifier) @_name)))
  arguments: (template_string
    (string_fragment) @injection.content)
  (#eq? @_name "styled")
  (#set! injection.language "css")
  (#set! injection.combined))

; el.innerHTML = `<b>x</b>` or el.innerHTML = '<b>x</b>'
(assignment_expression
  left: (member_expression
    property: (property_identifier) @_prop)
  right: [
    (template_string
      (string_fragment) @injection.content)
    (string
      (string_fragment) @injection.content)
  ]
  (#any-of? @_prop "outerHTML" "innerHTML")
  (#set! injection.language "html")
  (#set! injection.combined))

; @Component({ styles: [`...`] }) and @Component({ styles: `...` })
(decorator
  (call_expression
    function: (identifier) @_name
    arguments: (arguments
      (object
        (pair
          key: (property_identifier) @_prop
          value: [
            (array
              (template_string
                (string_fragment) @injection.content))
            (template_string
              (string_fragment) @injection.content)
          ]))))
  (#eq? @_name "Component")
  (#eq? @_prop "styles")
  (#set! injection.language "css")
  (#set! injection.combined))
