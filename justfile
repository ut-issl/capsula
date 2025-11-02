default:
    just --list

publish:
    cargo publish --workspace

lint:
    cargo clippy --workspace --all-targets --all-features
    cargo clippy --workspace --all-targets --no-default-features
    cargo fmt --check --all
    cargo doc --workspace --no-deps
    cargo check --workspace
