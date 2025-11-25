---
sidebar_position: 9
---

# Installation

## VS Code Extension

Ki is also available as a Visual Studio Code extension, at https://marketplace.visualstudio.com/items?itemName=ki-editor.ki-editor-vscode.

## Build from source with Nix (Recommended)
This is the most reliable installation method as all required dependencies,
including system dependencies, will be included automatically.

1. Ensure that [Nix: the package manager](https://nixos.org/download/) is installed.
2. Clone the project: 
```sh
git clone https://github.com/ki-editor/ki-editor.git
```
3. Run installation:
```sh
nix develop --command just install
```

## Build from source without Nix

1. Ensure the Rust toolchain is installed using [rustup.rs](https://rustup.rs/).
2. Use Rust 1.89.0:

```sh
rustup default 1.89.0
```

3. Install openssl's development package

```
# Ubuntu
sudo apt install libssl-dev

# Fedora
sudo dnf install openssl-devel
```

4. Clone the project:

```sh
git clone https://github.com/ki-editor/ki-editor.git
```

5. Run installation:

```sh
cd ki-editor
cargo install typeshare-cli  --git https://github.com/tomjw64/typeshare  --branch allow-override-for-disallowed-types
cargo install --locked --path .
```

6. The `ki` binary should be installed.

## Use nix flake [^1]

[^1]: This flake also provides a basic development environment for Ki Editor which can be enabled by `nix develop` command.

### 1. Install ki package

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

### 2. Build tree-sitter grammars

This step is optional as most of the Tree-sitter grammars are already linked at build time.


```sh
ki @ grammar fetch && ki @ grammar build
```
