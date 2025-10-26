publish:
    cargo publish -p capsula-core

    # Context crates
    cargo publish -p capsula-capture-cwd
    cargo publish -p capsula-capture-file
    cargo publish -p capsula-capture-git
    cargo publish -p capsula-capture-env
    cargo publish -p capsula-capture-command
    cargo publish -p capsula-capture-machine

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
