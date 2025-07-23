default:
    npm install
    @just tree-sitter-quickfix 
    @just fmt-check 
    @just build 
    @just vscode-build 
    @just lint 
    @just test 
    @just doc 
    
install:
    cargo install --locked --path .

fmt-check:
    @echo "Checking formating"
    cargo fmt --all -- --check
    
fmt:
	cargo fmt --all
	npm run format

build:
    @echo "Running cargo build..."
    cargo build --workspace --tests

watch-build:
    cargo watch --ignore ki-vscode --ignore ki-jetbrains -- cargo build

lint:
    @echo "Running cargo clippy..."
    cargo clippy --workspace -- -D warnings
    cargo clippy --tests -- -D warnings
    @just vscode-lint
    
vscode-lint:
    cd ki-vscode && ./node_modules/.bin/ts-unused-exports tsconfig.json --ignoreFiles="src/protocol/types"
    npm run lint
    
lint-fix:
	cargo clippy --workspace --tests --fix --allow-staged
	@just vscode-lint-fix

vscode-lint-fix:
	npm run lint:fix

test testname="":
    @echo "Running cargo test..."
    git config --get --global user.name  || git config --global user.name  Tester 
    git config --get --global user.email || git config --global user.email tester@gmail.com
    cargo test --workspace -- --nocapture -- {{testname}}
    
tree-sitter-quickfix:
    just -f tree_sitter_quickfix/justfile

doc:
    just -f docs/justfile

doc-serve:
	cd docs && just start

codecov:
	cargo tarpaulin --out html
    

watch-test testname:
	RUST_BACKTRACE=1 cargo watch --ignore ki-vscode --ignore ki-jetbrains --ignore 'tests/mock_repos/*' --ignore 'docs/static/*.json' -- cargo test --workspace  -- --nocapture -- {{testname}}
	
watch-clippy:
	RUST_BACKTRACE=1 cargo watch --ignore ki-vscode --ignore ki-jetbrains -- cargo clippy --workspace --tests
	
generate-recipes:
	just test "generate_recipes"
	
watch-generate-recipes:
	just watch-test "generate_recipes"

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