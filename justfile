default:
    just --list

publish:
    cargo publish -p capsula-core

    # Hook crates
    cargo publish -p capsula-capture-cwd
    cargo publish -p capsula-capture-file
    cargo publish -p capsula-capture-git-repo
    cargo publish -p capsula-capture-env
    cargo publish -p capsula-capture-command
    cargo publish -p capsula-capture-machine
    cargo publish -p capsula-notify-slack

    # Crates dependent on hook crates
    cargo publish -p capsula-registry
    cargo publish -p capsula-config
    cargo publish -p capsula-cli

lint:
    cargo clippy --workspace --all-targets --all-features
    cargo clippy --workspace --all-targets --no-default-features
    cargo fmt --check --all
    cargo doc --workspace --no-deps
    cargo check --workspace
