# Runtime Dockerfile for Agent Docs MCP Server with Claude
# Expects a prebuilt binary at build/http_server in the build context

# Use Claude image as base - it already has Node, npm, and Claude installed
FROM ghcr.io/5dlabs/claude:latest

# Switch to root to install additional dependencies
USER root

# Install runtime dependencies for the doc server
RUN apt-get update && apt-get install -y \
    libpq5 \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Set working directory
WORKDIR /app

# Copy prebuilt binaries from CI artifact
COPY --chown=node:node build/http_server /app/http_server
COPY --chown=node:node build/loader /app/loader
RUN chmod +x /app/http_server /app/loader

# Switch back to node user (already exists in base image)
USER node

# Configure environment
ENV RUST_LOG=info
ENV MCP_PORT=3001
ENV MCP_HOST=0.0.0.0
# Claude should already be in PATH from the base image
ENV CLAUDE_BINARY_PATH=claude

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

# Expose port
EXPOSE 3001

# Run the MCP server (exec form, absolute path)
CMD ["/app/http_server"]
