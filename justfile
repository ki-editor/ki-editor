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

build: install-typeshare
    @echo "Running cargo build..."
    cargo build --workspace --tests

watch-build: install-typeshare
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
	RUST_BACKTRACE=1 cargo watch --ignore ki-vscode --ignore ki-jetbrains -- cargo clippy --workspace --tests generate-recipes:
	just test "generate_recipes"
	
watch-generate-recipes:
	just watch-test "generate_recipes"

install-typeshare:
    cargo install --git https://github.com/tomjw64/typeshare

vscode-build: build
    cd ki-vscode && npm install

vscode-package:
    ./build-all-platforms.sh
    cd ki-vscode && npm run package

vscode-publish: vscode-package
    cd ki-vscode && npm run publish
    
# Install the locally build extension to VS Code
vscode-install: vscode-package
    code --install-extension ki-vscode/ki-editor-vscode-*.vsix
    
