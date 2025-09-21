publish:
    cargo publish -p capsula-core

    # Context crates
    cargo publish -p capsula-cwd-context
    cargo publish -p capsula-file-context
    cargo publish -p capsula-git-context

    # Crates dependent on context crates
    cargo publish -p capsula-registry
    cargo publish -p capsula-config
    cargo publish -p capsula-cli
