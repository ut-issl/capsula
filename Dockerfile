# Build stage
FROM rust:1@sha256:f2a0f2b3529c9bbbf5479d131611451a3cc3956d9a11374d6d4ba96f059c1dce AS builder

WORKDIR /usr/src/capsula-workspace

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Set SQLx to offline mode (uses .sqlx metadata from crates/capsula-server/.sqlx)
ENV SQLX_OFFLINE=true

# Build for release
RUN cargo build --release -p capsula-server

# Runtime stage
FROM debian:bookworm-slim@sha256:f06537653ac770703bc45b4b113475bd402f451e85223f0f2837acbf89ab020a

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

# Copy static files for web UI
COPY --from=builder /usr/src/capsula-workspace/crates/capsula-server/static /app/static
COPY --from=builder /usr/src/capsula-workspace/crates/capsula-server/templates /app/templates

# Copy migrations
COPY --from=builder /usr/src/capsula-workspace/crates/capsula-server/migrations /app/migrations

# Create storage directory with correct ownership
RUN mkdir -p /app/storage && chown capsula:capsula /app/storage

# Set working directory
WORKDIR /app

# Switch to non-root user
USER capsula

# Set static files directory for runtime
ENV CAPSULA_STATIC_DIR=/app/static
ENV CAPSULA_STORAGE_PATH=/app/storage

# Run the server
CMD ["capsula-server"]
