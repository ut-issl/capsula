# Build stage
FROM rust:1.99.1@sha256:8a4e90c7946b74626488b1446f712c0fe0ed1a596cbd3d871684bc9c5433ae07 AS builder

WORKDIR /usr/src/capsula-workspace

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Set SQLx to offline mode (uses .sqlx metadata from crates/capsula-server/.sqlx)
ENV SQLX_OFFLINE=true

# Build for release
RUN cargo build --release -p capsula-server

# Runtime stage
FROM debian:bookworm-20260316-slim@sha256:f06537653ac770703bc45b4b113475bd402f451e85223f0f2837acbf89ab020a

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user with UID 1000
RUN useradd --create-home --uid 1000 capsula

# Copy binary from builder
COPY --from=builder /usr/src/capsula-workspace/target/release/capsula-server /usr/local/bin/capsula-server

# Copy templates for web UI
COPY --from=builder /usr/src/capsula-workspace/crates/capsula-server/templates /app/templates

# Copy migrations
COPY --from=builder /usr/src/capsula-workspace/crates/capsula-server/migrations /app/migrations

# Set working directory
WORKDIR /app

# Switch to non-root user
USER capsula

# Run the server
CMD ["capsula-server"]
