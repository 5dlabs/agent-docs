# Multi-stage Dockerfile for Doc Server
FROM rust:1.83-bookworm as builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy dependency files first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/*/Cargo.toml ./crates/*/

# Create dummy source files to build dependencies
RUN mkdir -p crates/database/src crates/mcp/src crates/embeddings/src crates/doc-loader/src crates/llm/src crates/mcp/src/bin
RUN echo "fn main() {}" > crates/mcp/src/bin/http_server.rs
RUN echo "pub fn placeholder() {}" > crates/database/src/lib.rs
RUN echo "pub fn placeholder() {}" > crates/mcp/src/lib.rs  
RUN echo "pub fn placeholder() {}" > crates/embeddings/src/lib.rs
RUN echo "pub fn placeholder() {}" > crates/doc-loader/src/lib.rs
RUN echo "pub fn placeholder() {}" > crates/llm/src/lib.rs

# Build dependencies only
RUN cargo build --release --bin http_server -p doc-server-mcp

# Remove dummy source files
RUN rm -rf crates/*/src

# Copy actual source code
COPY crates/ crates/

# Build the application
RUN touch crates/mcp/src/bin/http_server.rs crates/*/src/lib.rs && \
    cargo build --release --bin http_server -p doc-server-mcp

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false -m -d /app docserver

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/http_server /usr/local/bin/http_server

# Change ownership
RUN chown -R docserver:docserver /app

# Switch to non-root user
USER docserver

# Expose port
EXPOSE 3001

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

# Set default environment variables
ENV RUST_LOG=info,doc_server=debug
ENV PORT=3001

# Run the application
CMD ["http_server"]