# Build stage
FROM rust:1.90 as builder

WORKDIR /usr/src/capsula-workspace

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Set SQLx to offline mode (uses .sqlx metadata from crates/capsula-server/.sqlx)
ENV SQLX_OFFLINE=true

# Build for release
RUN cargo build --release -p capsula-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 capsula && \
    mkdir -p /var/lib/capsula/storage && \
    chown -R capsula:capsula /var/lib/capsula

# Copy binary from builder
COPY --from=builder /usr/src/capsula-workspace/target/release/capsula-server /usr/local/bin/capsula-server

# Copy static files for web UI
COPY --from=builder /usr/src/capsula-workspace/crates/capsula-server/static /app/static
COPY --from=builder /usr/src/capsula-workspace/crates/capsula-server/templates /app/templates

# Set working directory
WORKDIR /app

# Switch to non-root user
USER capsula

# Run the server
CMD ["capsula-server"]
