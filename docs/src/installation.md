# Installation

Currently, only the "build from source" method is available.

## Build from source

1. Ensure the Rust toolchain is installed using [rustup.rs](https://rustup.rs/).
2. Use Rust 1.80.0:

```sh
rustup default 1.80.0
```

3. Clone the project:

```sh
git clone https://github.com/ki-editor/ki-editor.git
```

4. Run installation:

```sh
cd ki-editor
cargo install --locked --path .
```

5. The `ki` binary should be installed.
