publish:
    cargo publish -p capsula-core

    # Context crates
    cargo publish -p capsula-cwd-context
    cargo publish -p capsula-file-context
    cargo publish -p capsula-git-context
    cargo publish -p capsula-env-context
    cargo publish -p capsula-command-context
    cargo publish -p capsula-machine-context

    # Crates dependent on context crates
    cargo publish -p capsula-registry
    cargo publish -p capsula-config
    cargo publish -p capsula-cli

lint:
    cargo clippy --workspace --all-targets --all-features
    cargo clippy --workspace --all-targets --no-default-features
    cargo fmt --check --all
    cargo doc --workspace --no-deps
    cargo check --workspace
