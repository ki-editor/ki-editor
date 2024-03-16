default:
    @just fmt build clippy test

fmt:
    @echo "Running cargo fmt..."
    cargo fmt --all -- --check

build:
    @echo "Running cargo build..."
    cargo build --workspace --tests

clippy:
    @echo "Running cargo clippy..."
    cargo clippy --workspace -- -D warnings

test:
    @echo "Running cargo test..."
    git config --get --global user.name  || git config --global user.name  Tester 
    git config --get --global user.email || git config --global user.email tester@gmail.com
    cargo test --workspace
