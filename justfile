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
    docker compose -f ./crates/capsula-server/compose.yaml up -d

stop-db:
    docker compose -f ./crates/capsula-server/compose.yaml down

serve $DATABASE_URL="postgres://capsula:capsula_dev@localhost:5432/capsula" $RUST_LOG="info":
    cargo run -p capsula-server
