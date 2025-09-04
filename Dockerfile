# Multi-stage build to compile Rust binaries and run on Claude runtime image

# 1) Builder stage: compile Rust workspace
FROM --platform=linux/amd64 rust:1.79-bullseye AS builder

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

ENV SQLX_OFFLINE=true \
    RUSTFLAGS="-C target-cpu=x86-64-v3" \
    CARGO_TERM_COLOR=always

# Ensure problematic transitive dep stays pre-2024, then build
RUN cargo update -p base64ct --precise 1.7.3 && \
    cargo build --release --workspace

# 2) Runtime stage: Claude base image with Node and Claude installed
FROM --platform=linux/amd64 ghcr.io/5dlabs/claude:latest

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
