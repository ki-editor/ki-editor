# Ergo-L terminal input regression

Ki requests Kitty keyboard flags 31. Physical/base-layout keys drive positional
commands; the associated text field drives insertion and character search.
Ergo-L itself does not need changes.

The patched Ghostty must report ISO level-5 latch events with a base key:
`CSI 57454::111 u` on press and `CSI 57454::111;1:3 u` on release.
Unpatched terminals that consume ★ before sending input cannot be fixed by Ki.
This does not expose every intermediate dead key consumed by an IME.

Build and launch the locally patched pair:

```sh
cargo build -p ki
/mnt/data/src/zig/ghostty/zig-out/bin/ghostty --gtk-single-instance=false \
  -e /mnt/data/src/rust/ki-editor/target/debug/ki
```

Validation:

```sh
cargo test -p event
cargo test -p ki --lib keymap
cargo test --manifest-path vendor/crossterm/Cargo.toml --lib event::sys::unix::parse
python3 tests/terminal_ergol.py
```

The Linux PTY test runs the real Ki binary with an isolated Ergo-L configuration,
replays captured Ghostty CSI-u sequences, enters insertion using physical H,
and verifies the saved buffer. It checks ★, composed text, multi-scalar text,
repeat events and releases. It does not exercise a live Wayland compositor or
physical keyboard; those still need a manual test in the patched Ghostty.
