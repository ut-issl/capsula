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
