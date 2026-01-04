# Ki Editor

Multi-cursor structural editor. See [https://ki-editor.github.io/ki-editor/](https://ki-editor.github.io/ki-editor/).

Community chat room: 
- [ki-editor.zulip.com](https://ki-editor.zulipchat.com/join/zzhagqzl6wyzpqfeqxcsrkin/) (Recommended)

## Installation

At the moment, Ki is in heavy development, so you are encouraged to download the
[latest nightly build](https://github.com/ki-editor/ki-editor/releases/tag/nightly).

### Building from source

If you'd like to build Ki from source, you'll need a Rust compiler and a C compiler.
Check `rust-toolchain.toml` for the current version of Rust.

Then, all you need to do after cloning is:

```sh
cargo build --release
```

## Development

You are encouraged to use [direnv](https://github.com/direnv/direnv) and [Nix package manager](https://nixos.org/download/).

Common commands can be found in `justfile`.
