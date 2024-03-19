default:
    @just fmt-check build clippy test

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
    cargo clippy --workspace --tests -- -D warnings
    
clippy-fix:
	cargo clippy --workspace --tests --fix --allow-staged

test:
    @echo "Running cargo test..."
    git config --get --global user.name  || git config --global user.name  Tester 
    git config --get --global user.email || git config --global user.email tester@gmail.com
    cargo test --workspace
    

watch-test testname:
	RUST_BACKTRACE=1 cargo watch --ignore 'tests/mock_repos/*' -- cargo test --workspace  -- --nocapture -- {{testname}}
	
watch-clippy:
	RUST_BACKTRACE=1 cargo watch -- cargo clippy --workspace --tests
