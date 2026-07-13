# PR Guidelines

Always create a new branch before committing and creating a PR.

## Test plan

Only include manual testing steps. Do not list automated tests (unit tests, integration tests, etc.) — those are verified by CI.

## Coding style

Prefer functional style (iterators, `map`/`filter_map`/`fold`/`collect`, immutable bindings) over imperative loops with a mutable accumulator. Avoid `mut` unless the functional alternative would be significantly more complex, or `mut` is needed for a significant performance improvement. `mut` is fine, and expected, for things like trait-required signatures (`&mut Formatter`, `&mut SchemaGenerator`), fold accumulators, and required `&mut` buffers.
