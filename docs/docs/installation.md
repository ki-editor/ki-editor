# Installation

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

## Use nix flake [^1]

[^1]: This flake also provides a basic development environment for Ki Editor which can be enabled by `nix develop` command.

1. Install ki package

  - with `nix profile`:

```sh
nix profile install github:ki-editor/ki-editor
```

  - or as part of nix configuration, e.g.:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    ki-editor.url = "github:ki-editor/ki-editor";
  };

  outputs = { nixpkgs, ki-editor, ... }: {
    nixosConfigurations."«hostname»" = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
         { environment.systemPackages = [ ki-editor.packages.x86_64-linux.default ] }
         ./configuration.nix
      ];
    };
  };
}
```

2. Build tree-sitter grammars:

```sh
ki grammar fetch && ki grammar build
```
