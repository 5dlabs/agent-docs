# Runtime-only image expecting a pre-built binary at build/http_server in the context

FROM debian:bookworm-slim

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

WORKDIR /app

# Copy pre-built binary from CI into the image
COPY build/http_server /app/http_server

RUN chmod +x /app/http_server && \
    chown -R mcpuser:mcpuser /app

USER mcpuser

ENV RUST_LOG=info \
    MCP_PORT=3001 \
    MCP_HOST=0.0.0.0

EXPOSE 3001

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

CMD ["./http_server"]


