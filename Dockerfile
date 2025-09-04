# Multi-stage build to compile Rust binaries and run on Claude runtime image

# 1) Builder stage: compile Rust workspace
FROM rust:nightly-bullseye AS builder

WORKDIR /app

# Build dependencies required by some crates (e.g., git2)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    zlib1g-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Copy the full workspace
COPY . .

# Avoid any compile-time DB checks; we don't use sqlx macros requiring live DB
ENV SQLX_OFFLINE=true

# Build release binaries for server and loader
RUN cargo build --release --workspace

# 2) Runtime stage: Claude base image with Node and Claude installed
FROM ghcr.io/5dlabs/claude:latest

# Switch to root to install runtime deps and place binaries
USER root

RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

WORKDIR /app

# Copy compiled binaries from builder
COPY --from=builder /app/target/release/http_server /app/http_server
COPY --from=builder /app/target/release/loader /app/loader
RUN chown node:node /app/http_server /app/loader && \
    chmod +x /app/http_server /app/loader

# Drop privileges
USER node

# Configure environment
ENV RUST_LOG=info \
    MCP_PORT=3001 \
    MCP_HOST=0.0.0.0 \
    CLAUDE_BINARY_PATH=claude

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

# Expose port
EXPOSE 3001

# Run the MCP server
CMD ["/app/http_server"]
