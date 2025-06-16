default:
    @just tree-sitter-quickfix 
    @just fmt-check 
    @just build 
    @just build-vscode 
    @just clippy 
    @just test 
    @just doc 
    @just vscode-build
    
install:
    cargo install --locked --path .

fmt-check:
    @echo "Checking formating"
    cargo fmt --all -- --check
    
fmt:
	cargo fmt --all
	cd ki-vscode && npm run format

build:
    @echo "Running cargo build..."
    cargo build --workspace --tests

clippy:
    @echo "Running cargo clippy..."
    cargo clippy --workspace -- -D warnings
    cargo clippy --tests -- -D warnings
    
clippy-fix:
	cargo clippy --workspace --tests --fix --allow-staged

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
	RUST_BACKTRACE=1 cargo watch --ignore ki-vscode --ignore ki-jetbrains -- cargo clippy --workspace --tests generate-recipes:
	just test "generate_recipes"
	
watch-generate-recipes:
	just watch-test "generate_recipes"

watch-vscode-build:
    cargo watch --ignore ki-vscode --ignore ki-jetbrains -- cargo build --features vscode

vscode-build:
    cargo build --release --features vscode
    npm install
    cd ki-vscode && npm run format
    cd ki-vscode && npm run lint

vscode-package:
    ./build-all-platforms.sh
    cd ki-vscode && npm run package
    
# Install the locally build extension to VS Code
vscode-install: vscode-package
    code --install-extension ki-vscode/ki-editor-vscode-*.vsix
    
