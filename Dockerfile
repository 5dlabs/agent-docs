# Multi-stage Dockerfile for Agent Docs MCP Server
# Optimized for production Kubernetes deployment

# Build stage
FROM rust:1.88-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Prebuild dependencies to leverage Docker layer caching
RUN cargo fetch

# Build in release mode
RUN cargo build --release --bin http_server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false -m -d /app mcpuser

# Set working directory and change ownership
WORKDIR /app
RUN chown mcpuser:mcpuser /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/http_server /app/
COPY --chown=mcpuser:mcpuser --from=builder /app/target/release/http_server /app/

# Switch to non-root user
USER mcpuser

# Configure environment
ENV RUST_LOG=info
ENV MCP_PORT=3001
ENV MCP_HOST=0.0.0.0

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

# Expose port
EXPOSE 3001

# Run the MCP server
CMD ["./http_server"]