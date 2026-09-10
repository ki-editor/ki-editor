# Ki keyboard protocol changes

Source: https://github.com/vshylov/crossterm at
`0f086ab06bf6d577cdc0972d450d8a2c0dfe9363` (MIT, see LICENSE).
This copy preserves that fork's base-layout key support and adds:

- Kitty flag 16 (`REPORT_ASSOCIATED_TEXT`).
- Parsing the third CSI-u field into `KeyEvent.text: Option<String>`, including
  multiple Unicode scalar values and rejection of invalid scalars.
- Preservation of Shift when consuming an alternate shifted key.
- Owned key events (`Clone`, no longer `Copy`) to retain arbitrary committed text.
  Text, like base-layout metadata, is excluded from key equality and hashing.

Ki requests flags 31. With the patched Ghostty, Ergo-L's ISO level-5 latch is
reported as key 57454 with base-layout key 111 (physical O). Its release has the
same base key. Normal commands use this physical identity; text input uses the
separate committed text. Unenhanced events still use Ki's configured layout.

Validate the parser:

```sh
cargo test --manifest-path vendor/crossterm/Cargo.toml --lib event::sys::unix::parse
```

Remove this local copy when a maintained dependency provides equivalent support;
keep the raw Ghostty sequence regression tests when migrating.
