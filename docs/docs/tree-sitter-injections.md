# Tree-sitter injections

An injection query identifies regions of a host language that should be
highlighted with another grammar. Configure `injection_query` and
`injected_languages` on a language to enable this independently of any built-in
language integration.

For example, this workspace configuration treats Rust raw-string contents as JSON:

```json
{
  "languages": {
    "rust": {
      "injection_query": "((raw_string_literal (string_content) @injection.content) (#set! injection.language \"json\"))",
      "injected_languages": ["json"]
    }
  }
}
```

The query marks embedded text with `@injection.content` and supplies its language
using `#set! injection.language` or an `@injection.language` capture.
`injected_languages` lists the language configurations to load for highlighting.
Unknown language names are skipped.
