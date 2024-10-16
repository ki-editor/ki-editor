default:
    @just tree-sitter-quickfix 
    @just fmt-check 
    @just build 
    @just clippy 
    @just test 
    @just doc
    
install:
    cargo install --locked --path .

fmt-check:
    @echo "Checking formating"
    cargo fmt --all -- --check
    
fmt:
	cargo fmt --all

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
	RUST_BACKTRACE=1 cargo watch --ignore 'tests/mock_repos/*' --ignore 'docs/static/*.json' -- cargo test --workspace  -- --nocapture -- {{testname}}
	
watch-clippy:
	RUST_BACKTRACE=1 cargo watch -- cargo clippy --workspace --tests
	

generate-recipes:
	just test "generate_recipes"
	
watch-generate-recipes:
	just watch-test "generate_recipes"
