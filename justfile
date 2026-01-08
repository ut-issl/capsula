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

start-db:
    docker compose -f ./crates/capsula-server/compose.yaml up -d

stop-db:
    docker compose -f ./crates/capsula-server/compose.yaml down

serve $DATABASE_URL="postgres://capsula:capsula_dev@localhost:5432/capsula" $RUST_LOG="info":
    cargo run -p capsula-server
