# Runtime-only Dockerfile for Agent Docs MCP Server
# Expects a prebuilt binary at build/http_server in the build context

# Stage to get Claude binary
FROM ghcr.io/5dlabs/claude:latest as claude-provider

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

# Copy Claude binary from the claude-provider stage
COPY --from=claude-provider /usr/local/bin/claude /usr/local/bin/claude
RUN chmod +x /usr/local/bin/claude

# Copy prebuilt binary from CI artifact packaged in the build context
COPY --chown=mcpuser:mcpuser build/http_server /app/http_server
RUN chmod +x /app/http_server

# Switch to non-root user
USER mcpuser

# Configure environment
ENV RUST_LOG=info
ENV MCP_PORT=3001
ENV MCP_HOST=0.0.0.0
# Claude binary path for intelligent ingestion
ENV CLAUDE_BINARY_PATH=/usr/local/bin/claude

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

# Expose port
EXPOSE 3001

# Run the MCP server (exec form, absolute path)
CMD ["/app/http_server"]
