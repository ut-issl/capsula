envfile := justfile_directory() / ".env.server"

# Construct the DATABASE_URL from environment variables in the envfile

database_url := shell("dotenvx --quiet run -f \"$1\" -- bash -c 'echo \"postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@localhost:$POSTGRES_PORT/$POSTGRES_DB\"'", envfile)

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

# Generate JSON schema for capsula.toml (full config schema)
schema:
    cargo run -p capsula-cli -- schema --full --output capsula-schema.json
    @echo "Schema generated: capsula-schema.json"

# Generate hook-only schemas (for documentation)
schema-hooks:
    cargo run -p capsula-cli -- schema --output capsula-hooks-schema.json
    @echo "Hook schemas generated: capsula-hooks-schema.json"

# Validate capsula.toml against the schema
schema-validate: schema
    uvx check-jsonschema --schemafile capsula-schema.json capsula.toml
