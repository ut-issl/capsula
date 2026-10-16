# Evaluate database_url (which needs dotenvx) only in recipes that use it.

set lazy

envfile := justfile_directory() / ".env.server"

# Construct the DATABASE_URL from environment variables in the envfile

database_url := shell("dotenvx --quiet run -f \"$1\" -- bash -c 'echo \"postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@localhost:$POSTGRES_PORT/$POSTGRES_DB\"'", envfile)

default:
    just --list

publish:
    cargo publish --workspace

lint:
    CARGO_BUILD_WARNINGS=deny cargo clippy --workspace --all-targets --all-features
    CARGO_BUILD_WARNINGS=deny cargo clippy --workspace --all-targets --no-default-features
    cargo fmt --check --all
    CARGO_BUILD_WARNINGS=deny cargo doc --workspace --no-deps
    CARGO_BUILD_WARNINGS=deny cargo check --workspace
    cargo audit --deny warnings

test:
    cargo test --workspace

# Generate HTML coverage report
coverage:
    cargo llvm-cov --all-features --workspace --html

# Generate and open HTML coverage report in browser
coverage-open:
    cargo llvm-cov --all-features --workspace --html --open

# Generate coverage in LCOV format (for CI)
coverage-lcov:
    cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# Clean coverage artifacts
coverage-clean:
    cargo llvm-cov clean --workspace

start-db:
    dotenvx run -f {{ envfile }} -- docker compose up postgres --detach

stop-db:
    dotenvx run -f {{ envfile }} -- docker compose down

serve $RUST_LOG="info":
    cargo run -p capsula-server -- --database-url {{ database_url }}

[working-directory('crates/capsula-server')]
sqlx-prepare:
    cargo sqlx prepare --database-url {{ database_url }}

# Documentation (English and Japanese). See docs/README.md.

# Build the GitHub Pages artifact (site/) with the same steps as CI.
docs-build:
    rm -rf site
    zensical build --config-file zensical.toml --strict
    zensical build --config-file zensical.ja.toml --strict
    cp docs/site-root/index.html site/
    cp site/en/404.html site/

# English live preview.
docs-serve:
    zensical serve

# Japanese live preview.
docs-serve-ja:
    zensical serve --config-file zensical.ja.toml

# Production-like preview of both editions: http://127.0.0.1:8000/capsula/
docs-preview: docs-build
    rm -rf target/docs-preview
    mkdir -p target/docs-preview
    cp -R site target/docs-preview/capsula
    python3 -m http.server --directory target/docs-preview --bind 127.0.0.1 8000
