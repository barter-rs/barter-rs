# Multi-stage build for optimal image size
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/barter

# Copy manifest files
COPY Cargo.toml Cargo.lock ./
COPY barter/Cargo.toml ./barter/
COPY barter-data/Cargo.toml ./barter-data/
COPY barter-execution/Cargo.toml ./barter-execution/
COPY barter-instrument/Cargo.toml ./barter-instrument/
COPY barter-integration/Cargo.toml ./barter-integration/
COPY barter-macro/Cargo.toml ./barter-macro/
COPY barter-strategy/Cargo.toml ./barter-strategy/

# Create dummy source files to cache dependencies
RUN mkdir -p barter/src && echo "fn main() {}" > barter/src/main.rs
RUN mkdir -p barter-data/src && echo "" > barter-data/src/lib.rs
RUN mkdir -p barter-execution/src && echo "" > barter-execution/src/lib.rs
RUN mkdir -p barter-instrument/src && echo "" > barter-instrument/src/lib.rs
RUN mkdir -p barter-integration/src && echo "" > barter-integration/src/lib.rs
RUN mkdir -p barter-macro/src && echo "" > barter-macro/src/lib.rs
RUN mkdir -p barter-strategy/src && echo "" > barter-strategy/src/lib.rs

# Build dependencies
RUN cargo build --release && rm -rf target/release/deps/barter*

# Copy actual source code
COPY . .

# Touch source files to ensure rebuild
RUN find . -name "*.rs" -exec touch {} \;

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r barter && useradd -r -g barter barter

# Copy binary from builder
COPY --from=builder /usr/src/barter/target/release/barter-strategy /usr/local/bin/barter-strategy

# Copy configuration and scripts
COPY --from=builder /usr/src/barter/barter-strategy/examples /opt/barter/examples
COPY --from=builder /usr/src/barter/docs /opt/barter/docs
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Create necessary directories
RUN mkdir -p /opt/barter/config /opt/barter/data /opt/barter/logs && \
    chown -R barter:barter /opt/barter

# Switch to non-root user
USER barter

WORKDIR /opt/barter

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["echo", "healthy"]

# Entrypoint
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

# Default command
CMD ["barter-strategy"]