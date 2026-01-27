default:
    npm install
    @just check 
    @just build-all 
    @just lint 
    @just test 
    @just doc 

check: build check-typeshare fmt-check lint 
    
build-all: tree-sitter-quickfix build vscode-build
    
install:
    rm -r ~/.cache/ki/zed-themes || echo "ok" 
    cargo install --locked --path .

fmt-check:
    @echo "Checking formating"
    cargo fmt --all -- --check
    alejandra --exclude ./nvim-treesitter-highlight-queries/nvim-treesitter/ --check ./
    
fmt:
	cargo fmt --all
	npm run format
	alejandra --exclude ./nvim-treesitter-highlight-queries/nvim-treesitter/ ./

build:
    @echo "Running cargo build..."
    cargo build --workspace --tests

watch-build:
    cargo watch --ignore ki-vscode --ignore ki-jetbrains -- cargo build

# Note: not removing the generated file on error here is intentional,
# partially because it is more annoying.
check-typeshare:
    typeshare ki-protocol-types/src --lang typescript -o ki-vscode/src/protocol/types.ts2
    cd ki-vscode/src/protocol && cmp types.ts types.ts2 && rm types.ts2
    
    typeshare ki-protocol-types/src --lang kotlin -o ki-jetbrains/src/kotlin/protocol/Types.kt2
    cd ki-jetbrains/src/kotlin/protocol && cmp Types.kt2 Types.kt && rm Types.kt2

update-typeshare:
    typeshare ki-protocol-types/src --lang typescript -o ki-vscode/src/protocol/types.ts
    typeshare ki-protocol-types/src --lang kotlin -o ki-jetbrains/src/kotlin/protocol/Types.kt

lint:
    @echo "Running cargo clippy..."
    cargo clippy --workspace -- -D warnings
    cargo clippy --tests -- -D warnings
    cargo machete
    npm install
    npm run lint
    @just vscode-lint
    
[working-directory: 'ki-vscode']
vscode-lint:
    npm install
    ./node_modules/.bin/ts-unused-exports tsconfig.json --ignoreFiles="src/protocol/types"
    
lint-fix:
	cargo clippy --workspace --tests --fix --allow-staged --allow-dirty
	@just vscode-lint-fix

vscode-lint-fix:
	npm run lint:fix
	
test-setup:
    git config --get --global user.name  || git config --global user.name  Tester 
    git config --get --global user.email || git config --global user.email tester@gmail.com

test testname="": test-setup
    echo "Running cargo nextest..."
    cargo nextest run --workspace -- --skip 'doc_assets_' {{testname}}
    
tree-sitter-quickfix:
    just -f tree_sitter_quickfix/justfile

doc-assets testname="": test-setup
    cargo nextest run --workspace -- 'doc_assets_' {{testname}}

doc-assets-generate-keymaps:
    cargo test -- doc_assets_export_keymaps_json


check-config-schema:
    #!/bin/sh
    set -e
    set -x
    cargo build 
    cargo test -- doc_assets_export_json_schemas
    if ! git diff --exit-code docs/static/app_config_json_schema.json; then
        echo "❌ Config schema is out of date!"
        echo "Please run 'just check-config-schema' and commit 'docs/static/app_config_json_schema.json'."
        exit 1
    fi
    
# This command helps you locate the actual recipe that is failing
doc-assets-get-recipes-error:
    just doc-assets generate_recipes > /dev/null

doc: doc-assets
    just check-config-schema
    just -f docs/justfile

doc-serve:
	cd docs && just start

codecov:
	cargo tarpaulin --out html
    

watch-test testname:
	RUST_BACKTRACE=1 cargo watch --ignore ki-vscode --ignore ki-jetbrains --ignore 'mock_repos/*' --ignore 'docs/static/*.json' -- cargo test --workspace  -- {{testname}}
	
watch-clippy:
	RUST_BACKTRACE=1 cargo watch --ignore ki-vscode --ignore ki-jetbrains -- cargo clippy --workspace --tests
	
vscode-build: build
    cd ki-vscode && npm install

vscode-package:
    @just vscode-build-binaries
    cd ki-vscode && npm run package

vscode-publish: vscode-package
    cd ki-vscode && npm run publish
    
# Build and install the locally build VS Code extension
vscode-install:
    rm ki-vscode/ki-editor-vscode-*.vsix || true
    cargo build --release 
    cp target/release/ki ./ki-vscode/dist/bin/ki-darwin-arm64  
    cd ki-vscode && npm install  
    cd ki-vscode && npm run package  
    rm -rf ~/.vscode/extensions/ki-editor.ki-editor-vscode-0.0.*/** 
    code --install-extension ki-vscode/ki-editor-vscode-*.vsix 

# Build and package Ki Editor binary for a specific platform
_vscode-build-binary platform nix_target output_name binary_name="ki":
    #!/usr/bin/env bash
    mkdir -p ki-vscode/dist/bin
    nix build -L .#{{nix_target}}
    BUILD_PATH=$(readlink -f result 2>/dev/null || echo "")
    if [[ -n "${BUILD_PATH}" && -f "${BUILD_PATH}/bin/{{binary_name}}" ]]; then
        cp -f "${BUILD_PATH}/bin/{{binary_name}}" "ki-vscode/dist/bin/{{output_name}}"
        if [[ "{{binary_name}}" == "ki" ]]; then
            chmod +x "ki-vscode/dist/bin/{{output_name}}"
        fi
        echo "Copied {{platform}} build"
    else
        echo "{{platform}} build not found"
        exit 1
    fi
    rm -f result

# Build and package Ki Editor binary for macOS ARM64
vscode-build-binary-mac-arm64:
    just _vscode-build-binary "aarch64-darwin" "aarch64-darwin" "ki-darwin-arm64"

# Build and package Ki Editor binary for Linux x86_64
vscode-build-binary-linux-x64:
    just _vscode-build-binary "x86_64-linux-musl" "x86_64-linux-musl" "ki-linux-x64"

# Build and package Ki Editor binary for Linux ARM64
vscode-build-binary-linux-arm64:
    just _vscode-build-binary "aarch64-linux" "aarch64-linux" "ki-linux-arm64"

# Build and package Ki Editor binary for Windows x86_64
vscode-build-binary-windows-x64:
    just _vscode-build-binary "x86_64-windows-gnu" "x86_64-windows-gnu" "ki-win32-x64.exe" "ki.exe"

# Build Ki Editor for all platforms and copy binaries to ki-vscode/dist/bin
vscode-build-binaries:
    just vscode-build-binary-mac-arm64
    just vscode-build-binary-linux-x64
    just vscode-build-binary-linux-arm64
    just vscode-build-binary-windows-x64
    ls -la ki-vscode/dist/bin/
    echo "Done!"

profile args="":
    cargo build --release
    samply record ./target/release/ki {{args}}
