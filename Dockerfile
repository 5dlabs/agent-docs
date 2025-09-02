# Multi-stage build for Agent Docs MCP Server (spec-compliant JSON-RPC HTTP)

# Build stage
FROM rust:1.79-bullseye AS builder

WORKDIR /workspace

# Cache deps first
COPY Cargo.toml Cargo.lock ./
COPY db/Cargo.toml db/Cargo.toml
COPY embed/Cargo.toml embed/Cargo.toml
COPY llm/Cargo.toml llm/Cargo.toml
COPY loader/Cargo.toml loader/Cargo.toml
COPY mcp/Cargo.toml mcp/Cargo.toml

RUN mkdir -p db/src embed/src llm/src loader/src mcp/src && \
    echo "fn main(){}" > mcp/src/main.rs && \
    cargo build -p mcp --bin http_server --release || true

# Copy source
COPY . .

# Build release binary
RUN cargo build -p mcp --bin http_server --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN useradd -r -s /bin/false -m -d /app mcpuser

# Set working directory and change ownership
WORKDIR /app
RUN chown mcpuser:mcpuser /app

# Copy binary from builder
COPY --from=builder /workspace/target/release/http_server /app/http_server
RUN chmod +x /app/http_server

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

# Run the MCP server (exec form, absolute path)
CMD ["/app/http_server"]
